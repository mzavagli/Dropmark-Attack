use linux_api::errno::Errno;
use linux_api::time::{ClockId, ClockNanosleepFlags, ITimerId};
use log::*;
use shadow_shim_helper_rs::emulated_time::EmulatedTime;
use shadow_shim_helper_rs::simulation_time::SimulationTime;
use shadow_shim_helper_rs::syscall_types::ForeignPtr;
use syscall_logger::log_syscall;

use crate::core::worker::Worker;
use crate::host::syscall::handler::{SyscallContext, SyscallHandler};
use crate::host::syscall::types::{SyscallError, SyscallResult};
use crate::host::timer::Timer;

fn itimerval_from_timer(timer: &Timer) -> linux_api::time::itimerval {
    linux_api::time::itimerval {
        it_interval: timer
            .expire_interval()
            .unwrap_or(SimulationTime::ZERO)
            .try_into()
            .unwrap(),
        it_value: timer
            .remaining_time()
            .unwrap_or(SimulationTime::ZERO)
            .try_into()
            .unwrap(),
    }
}

impl SyscallHandler {
    #[log_syscall(/* rv */ std::ffi::c_int, /* which */ linux_api::time::ITimerId, /*curr_value*/ *const std::ffi::c_void)]
    pub fn getitimer(
        ctx: &mut SyscallContext,
        which: std::ffi::c_int,
        curr_value_ptr: ForeignPtr<linux_api::time::itimerval>,
    ) -> SyscallResult {
        let Ok(which) = ITimerId::try_from(which) else {
            debug!("Bad itimerid {which}");
            return Err(Errno::EINVAL.into());
        };

        if which != ITimerId::ITIMER_REAL {
            warn_once_then_debug!("Timer type {which:?} unsupported");
            return Err(Errno::EINVAL.into());
        }

        let itimerval = itimerval_from_timer(&ctx.objs.process.realtime_timer_borrow());
        ctx.objs
            .process
            .memory_borrow_mut()
            .write(curr_value_ptr, &itimerval)?;

        Ok(0.into())
    }

    #[log_syscall(/* rv */ std::ffi::c_int, /* which */ linux_api::time::ITimerId,
                  /* new_value */ *const std::ffi::c_void, /* old_value */ *const std::ffi::c_void)]
    pub fn setitimer(
        ctx: &mut SyscallContext,
        which: std::ffi::c_int,
        new_value_ptr: ForeignPtr<linux_api::time::itimerval>,
        old_value_ptr: ForeignPtr<linux_api::time::itimerval>,
    ) -> SyscallResult {
        let Ok(which) = ITimerId::try_from(which) else {
            debug!("Bad itimerid {which}");
            return Err(Errno::EINVAL.into());
        };

        if which != ITimerId::ITIMER_REAL {
            warn_once_then_debug!("Timer type {which:?} unsupported");
            return Err(Errno::EINVAL.into());
        }

        if !old_value_ptr.is_null() {
            let itimerval = itimerval_from_timer(&ctx.objs.process.realtime_timer_borrow());
            ctx.objs
                .process
                .memory_borrow_mut()
                .write(old_value_ptr, &itimerval)?;
        }

        let new_value = ctx.objs.process.memory_borrow().read(new_value_ptr)?;
        let new_value_value =
            SimulationTime::try_from(new_value.it_value).map_err(|_| Errno::EINVAL)?;
        let new_value_interval =
            SimulationTime::try_from(new_value.it_interval).map_err(|_| Errno::EINVAL)?;

        if new_value_value == SimulationTime::ZERO {
            ctx.objs.process.realtime_timer_borrow_mut().disarm();
        } else {
            ctx.objs.process.realtime_timer_borrow_mut().arm(
                ctx.objs.host,
                Worker::current_time().unwrap() + new_value_value,
                new_value_interval
                    .is_positive()
                    .then_some(new_value_interval),
            );
        }

        Ok(0.into())
    }

    #[log_syscall(/* rv */ std::ffi::c_int, /* clock_id */ linux_api::time::ClockId,
                  /* res */ *const std::ffi::c_void)]
    pub fn clock_getres(
        ctx: &mut SyscallContext,
        clock_id: linux_api::time::linux___kernel_clockid_t,
        res_ptr: ForeignPtr<linux_api::time::timespec>,
    ) -> Result<std::ffi::c_int, SyscallError> {
        // Make sure we have a valid clock id.
        ClockId::try_from(clock_id).map_err(|_| Errno::EINVAL)?;

        // All clocks have nanosecond resolution.
        if !res_ptr.is_null() {
            let res_time = linux_api::time::timespec::try_from(SimulationTime::NANOSECOND).unwrap();
            ctx.objs
                .process
                .memory_borrow_mut()
                .write(res_ptr, &res_time)?;
        }

        Ok(0)
    }

    #[log_syscall(/* rv */ std::ffi::c_int,
        /* clock_id */ linux_api::time::ClockId,
        /* flags */ linux_api::time::ClockNanosleepFlags,
        /* request */ *const linux_api::time::timespec,
        /* remain */ *const std::ffi::c_void)]
    pub fn clock_nanosleep(
        ctx: &mut SyscallContext,
        clock_id: linux_api::time::linux___kernel_clockid_t,
        flags: std::ffi::c_int,
        request_ptr: ForeignPtr<linux_api::time::timespec>,
        remain_ptr: ForeignPtr<linux_api::time::timespec>,
    ) -> Result<std::ffi::c_int, SyscallError> {
        let clock_id = ClockId::try_from(clock_id).map_err(|_| Errno::EINVAL)?;

        // Check for clock_ids that specifically support nanosleep.
        if [
            ClockId::CLOCK_MONOTONIC,
            ClockId::CLOCK_REALTIME,
            ClockId::CLOCK_BOOTTIME,
            ClockId::CLOCK_TAI,
            ClockId::CLOCK_REALTIME_ALARM,
            ClockId::CLOCK_BOOTTIME_ALARM,
        ]
        .contains(&clock_id)
        {
            // Simulated in Shadow; Linux allows unspec bitflags, but not for the *ALARM clocks.
            let allow_unspec_bitflags =
                ![ClockId::CLOCK_REALTIME_ALARM, ClockId::CLOCK_BOOTTIME_ALARM].contains(&clock_id);
            Self::nanosleep_helper(ctx, flags, request_ptr, remain_ptr, allow_unspec_bitflags)
        } else if [ClockId::CLOCK_THREAD_CPUTIME_ID].contains(&clock_id) {
            // Invalid in Linux.
            log::debug!("Invalid clock id {clock_id:?}.",);
            Err(Errno::EINVAL.into())
        } else if [
            ClockId::CLOCK_MONOTONIC_RAW,
            ClockId::CLOCK_REALTIME_COARSE,
            ClockId::CLOCK_MONOTONIC_COARSE,
        ]
        .contains(&clock_id)
        {
            // Not supported in Linux.
            log::debug!("Clock id {clock_id:?} unsupported for clock_nanosleep.",);
            Err(Errno::ENOTSUP.into())
        } else if [ClockId::CLOCK_PROCESS_CPUTIME_ID].contains(&clock_id) {
            // Supported in Linux, not in Shadow.
            warn_once_then_debug!("Clock id {clock_id:?} unsupported in Shadow.",);
            Err(Errno::ENOTSUP.into())
        } else {
            log::debug!("Unknown clock id {clock_id:?}.");
            Err(Errno::EINVAL.into())
        }
    }

    #[log_syscall(/* rv */ std::ffi::c_int, /* req */ *const linux_api::time::timespec,
                  /* rem */ *const std::ffi::c_void)]
    pub fn nanosleep(
        ctx: &mut SyscallContext,
        req: ForeignPtr<linux_api::time::timespec>,
        rem: ForeignPtr<linux_api::time::timespec>,
    ) -> Result<std::ffi::c_int, SyscallError> {
        Self::nanosleep_helper(ctx, 0, req, rem, false)
    }

    fn nanosleep_helper(
        ctx: &mut SyscallContext,
        flags: std::ffi::c_int,
        request_ptr: ForeignPtr<linux_api::time::timespec>,
        remain_ptr: ForeignPtr<linux_api::time::timespec>,
        allow_unspec_bitflags: bool,
    ) -> Result<std::ffi::c_int, SyscallError> {
        let request = ctx.objs.process.memory_borrow().read(request_ptr)?;
        let request_time = SimulationTime::try_from(request).or(Err(Errno::EINVAL))?;
        let flags = if allow_unspec_bitflags {
            ClockNanosleepFlags::from_bits_truncate(flags)
        } else {
            ClockNanosleepFlags::from_bits(flags).ok_or(Errno::EINVAL)?
        };

        let now = Worker::current_time().unwrap();

        // The requested wakeup time may be absolute or relative.
        let abs_wakeup_time = if flags.contains(ClockNanosleepFlags::TIMER_ABSTIME) {
            EmulatedTime::UNIX_EPOCH + request_time
        } else {
            now + request_time
        };

        // A wakeup time in the past means we return without sleeping.
        if abs_wakeup_time <= now {
            return Ok(0);
        }

        // Condition will exist after a wakeup.
        let Some(cond) = ctx.objs.thread.syscall_condition() else {
            // Didn't sleep yet; block the thread now.
            return Err(SyscallError::new_blocked_until(abs_wakeup_time, false));
        };

        // Woke up from sleep. We must have set a timeout to sleep.
        let expected_wakeup_time = cond.timeout().unwrap();

        if expected_wakeup_time <= now {
            // Successful sleep and wakeup!
            Ok(0)
        } else {
            // Possibly write out the remaining time until the expected wakeup.
            if !remain_ptr.is_null() && !flags.contains(ClockNanosleepFlags::TIMER_ABSTIME) {
                let remain_time =
                    linux_api::time::timespec::try_from(expected_wakeup_time - now).unwrap();
                ctx.objs
                    .process
                    .memory_borrow_mut()
                    .write(remain_ptr, &remain_time)?;
            }

            // Encodes that we were interrupted but will return EINTR to the plugin.
            Err(SyscallError::new_interrupted(false))
        }
    }
}
