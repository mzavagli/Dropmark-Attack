use crate::bindings;

pub const RTM_GETADDR: u16 = bindings::LINUX_RTM_GETADDR as u16;
pub const RTM_GETLINK: u16 = bindings::LINUX_RTM_GETLINK as u16;

pub const RTMGRP_IPV4_IFADDR: u32 = bindings::LINUX_RTMGRP_IPV4_IFADDR;
pub const RTMGRP_IPV6_IFADDR: u32 = bindings::LINUX_RTMGRP_IPV6_IFADDR;
