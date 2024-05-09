use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};

use crate::core::worker::Worker;
use crate::cshadow as c;
use crate::host::memory_manager::MemoryManager;
use crate::host::network::interface::FifoPacketPriority;
use crate::host::syscall::io::IoVec;
use crate::utility::pcap_writer::PacketDisplay;

use linux_api::errno::Errno;
use shadow_shim_helper_rs::simulation_time::SimulationTime;
use shadow_shim_helper_rs::util::SyncSendPointer;

#[repr(i32)]
pub enum PacketStatus {
    SndCreated = c::_PacketDeliveryStatusFlags_PDS_SND_CREATED,
    SndTcpEnqueueThrottled = c::_PacketDeliveryStatusFlags_PDS_SND_TCP_ENQUEUE_THROTTLED,
    SndTcpEnqueueRetransmit = c::_PacketDeliveryStatusFlags_PDS_SND_TCP_ENQUEUE_RETRANSMIT,
    SndTcpDequeueRetransmit = c::_PacketDeliveryStatusFlags_PDS_SND_TCP_DEQUEUE_RETRANSMIT,
    SndTcpRetransmitted = c::_PacketDeliveryStatusFlags_PDS_SND_TCP_RETRANSMITTED,
    SndSocketBuffered = c::_PacketDeliveryStatusFlags_PDS_SND_SOCKET_BUFFERED,
    SndInterfaceSent = c::_PacketDeliveryStatusFlags_PDS_SND_INTERFACE_SENT,
    InetSent = c::_PacketDeliveryStatusFlags_PDS_INET_SENT,
    InetDropped = c::_PacketDeliveryStatusFlags_PDS_INET_DROPPED,
    RouterEnqueued = c::_PacketDeliveryStatusFlags_PDS_ROUTER_ENQUEUED,
    RouterDequeued = c::_PacketDeliveryStatusFlags_PDS_ROUTER_DEQUEUED,
    RouterDropped = c::_PacketDeliveryStatusFlags_PDS_ROUTER_DROPPED,
    RcvInterfaceReceived = c::_PacketDeliveryStatusFlags_PDS_RCV_INTERFACE_RECEIVED,
    RcvInterfaceDropped = c::_PacketDeliveryStatusFlags_PDS_RCV_INTERFACE_DROPPED,
    RcvSocketProcessed = c::_PacketDeliveryStatusFlags_PDS_RCV_SOCKET_PROCESSED,
    RcvSocketDropped = c::_PacketDeliveryStatusFlags_PDS_RCV_SOCKET_DROPPED,
    RcvTcpEnqueueUnordered = c::_PacketDeliveryStatusFlags_PDS_RCV_TCP_ENQUEUE_UNORDERED,
    RcvSocketBuffered = c::_PacketDeliveryStatusFlags_PDS_RCV_SOCKET_BUFFERED,
    RcvSocketDelivered = c::_PacketDeliveryStatusFlags_PDS_RCV_SOCKET_DELIVERED,
    Destroyed = c::_PacketDeliveryStatusFlags_PDS_DESTROYED,
    RelayCached = c::_PacketDeliveryStatusFlags_PDS_RELAY_CACHED,
    RelayForwarded = c::_PacketDeliveryStatusFlags_PDS_RELAY_FORWARDED,
}

pub struct PacketRc {
    c_ptr: SyncSendPointer<c::Packet>,
}

impl std::fmt::Debug for PacketRc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet").finish_non_exhaustive()
    }
}

impl PartialEq for PacketRc {
    fn eq(&self, other: &Self) -> bool {
        self.c_ptr.ptr() == other.c_ptr.ptr()
    }
}

impl Eq for PacketRc {}

/// Clone the reference to the packet.
impl Clone for PacketRc {
    fn clone(&self) -> Self {
        let ptr = self.borrow_inner();
        unsafe { c::packet_ref(ptr) }
        PacketRc::from_raw(ptr)
    }
}

impl PacketRc {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // creating a new packet shouldn't require the host, so for now we'll get the host from the
        // worker rather than require it as an argument
        Self::from_raw(Worker::with_active_host(|host| unsafe { c::packet_new(host) }).unwrap())
    }

    #[cfg(test)]
    /// Creates an empty packet for unit tests.
    pub fn mock_new() -> PacketRc {
        let c_ptr = unsafe { c::packet_new_inner(1, 1) };
        unsafe { c::packet_setMock(c_ptr) };
        PacketRc::from_raw(c_ptr)
    }

    /// Set TCP headers for this packet. Will panic if the packet already has a header.
    pub fn set_tcp(&mut self, header: &tcp::TcpHeader) {
        let selective_acks = header
            .selective_acks
            .as_ref()
            .map(AsRef::as_ref)
            .unwrap_or(&[]);

        // the tcp header allows for a max of 4 begin/end pairs
        assert!(selective_acks.len() <= 4);

        let mut selective_acks_glist = std::ptr::null_mut();

        for sack in selective_acks {
            for val in [sack.0, sack.1] {
                // integer to pointer cast
                let val = val as *mut libc::c_void;
                selective_acks_glist = unsafe { c::g_list_append(selective_acks_glist, val) };
            }
        }

        // TODO: not sure if linux uses milliseconds, but it probably doesn't matter as long as we
        // convert it back to a u32 the same way when receiving packets
        let timestamp = SimulationTime::from_millis(header.timestamp.unwrap_or(0).into());
        let timestamp_echo = SimulationTime::from_millis(header.timestamp_echo.unwrap_or(0).into());

        unsafe {
            c::packet_setTCP(
                self.c_ptr.ptr(),
                to_legacy_tcp_flags(header.flags),
                u32::from(*header.src().ip()).to_be(),
                header.src().port().to_be(),
                u32::from(*header.dst().ip()).to_be(),
                header.dst().port().to_be(),
                header.seq,
            );

            c::packet_updateTCP(
                self.c_ptr.ptr(),
                header.ack,
                selective_acks_glist,
                header.window_size.into(),
                header.window_scale.unwrap_or(0),
                header.window_scale.is_some(),
                timestamp.into(),
                timestamp_echo.into(),
            );
        }

        // the C packet should make a copy of the glist, so we can free ours
        unsafe { c::g_list_free(selective_acks_glist) };
    }

    pub fn get_tcp(&self) -> Option<tcp::TcpHeader> {
        let header = unsafe { c::packet_getTCPHeader(self.c_ptr.ptr()) };
        let header = unsafe { header.as_ref()? };

        // TODO: not sure if linux uses milliseconds, but it probably doesn't matter as long as we
        // converted it to a SimulationTime the same way when sending the packet
        let timestamp = SimulationTime::from_c_simtime(header.timestampValue)
            .unwrap()
            .as_millis();
        let timestamp_echo = SimulationTime::from_c_simtime(header.timestampEcho)
            .unwrap()
            .as_millis();

        // need to get the selective acks from a glist, so we'll panic a lot here to simplify things

        let mut selective_acks = header.selectiveACKs;

        // TODO: this selective ack code is untested until the new tcp code uses sacks, so it has
        // not been checked for memory safety issues and there are probably bugs
        let mut sack_iter = std::iter::from_fn(move || {
            if selective_acks.is_null() {
                return None;
            }

            let rv = unsafe { (*selective_acks).data } as u64;
            selective_acks = unsafe { (*selective_acks).next };
            Some(u32::try_from(rv).unwrap())
        });

        let sack_iter = std::iter::from_fn(move || {
            // we expect the packet sack list to have a length divisible by 2
            let begin = sack_iter.next()?;
            let end = sack_iter.next().unwrap();

            Some((begin, end))
        });

        let selective_acks: Vec<_> = sack_iter.collect();
        let selective_acks = tcp::util::SmallArrayBackedSlice::new(&selective_acks).unwrap();

        // the C packet doesn't have the distinction between no sack option or a sack option of
        // length 0, so we'll assume that an empty list is the same as no list
        let selective_acks = if !selective_acks.is_empty() {
            Some(selective_acks)
        } else {
            None
        };

        let window_scale = header.windowScaleSet.then_some(header.windowScale);

        let src_ip = Ipv4Addr::from(u32::from_be(header.sourceIP));
        let src_port = u16::from_be(header.sourcePort);

        let dst_ip = Ipv4Addr::from(u32::from_be(header.destinationIP));
        let dst_port = u16::from_be(header.destinationPort);

        Some(tcp::TcpHeader {
            ip: tcp::Ipv4Header {
                src: src_ip,
                dst: dst_ip,
            },
            flags: from_legacy_tcp_flags(header.flags),
            src_port,
            dst_port,
            seq: header.sequence,
            ack: header.acknowledgment,
            window_size: header.window.try_into().unwrap(),
            selective_acks,
            window_scale,
            timestamp: Some(timestamp.try_into().unwrap()),
            timestamp_echo: Some(timestamp_echo.try_into().unwrap()),
        })
    }

    /// Set UDP headers for this packet. Will panic if the packet already has a header.
    pub fn set_udp(&mut self, src: SocketAddrV4, dst: SocketAddrV4) {
        unsafe {
            c::packet_setUDP(
                self.c_ptr.ptr(),
                c::ProtocolUDPFlags_PUDP_NONE,
                u32::from(*src.ip()).to_be(),
                src.port().to_be(),
                u32::from(*dst.ip()).to_be(),
                dst.port().to_be(),
            )
        };
    }

    /// Set the packet payload. Will panic if the packet already has a payload.
    pub fn set_payload(&mut self, payload: &[u8], priority: FifoPacketPriority) {
        unsafe {
            c::packet_setPayloadFromShadow(
                self.c_ptr.ptr(),
                payload.as_ptr() as *const libc::c_void,
                payload.len().try_into().unwrap(),
                priority,
            )
        }
    }

    /// Copy the packet payload to a buffer. Will truncate if the buffer is not large enough.
    pub fn get_payload(&self, buffer: &mut [u8]) -> usize {
        unsafe {
            c::packet_copyPayloadShadow(
                self.c_ptr.ptr(),
                0,
                buffer.as_mut_ptr().cast(),
                buffer.len().try_into().unwrap(),
            )
            .try_into()
            .unwrap()
        }
    }

    /// Copy the payload to the managed process. Even if this returns an error, some unspecified
    /// number of bytes may have already been copied.
    pub fn copy_payload<'a>(
        &self,
        iovs: impl IntoIterator<Item = &'a IoVec>,
        mem: &mut MemoryManager,
    ) -> Result<usize, linux_api::errno::Errno> {
        let iovs = iovs.into_iter();
        let mut bytes_copied = 0;

        for iov in iovs {
            let rv = unsafe {
                c::packet_copyPayloadWithMemoryManager(
                    self.c_ptr.ptr(),
                    bytes_copied,
                    iov.base.cast::<()>(),
                    iov.len.try_into().unwrap(),
                    mem,
                )
            };

            if rv < 0 {
                return Err(Errno::try_from(-rv).unwrap());
            }

            let rv = rv as u64;

            if rv == 0 && iov.len != 0 {
                // no more payload bytes to copy
                break;
            }

            bytes_copied += rv;
        }

        Ok(bytes_copied.try_into().unwrap())
    }

    pub fn total_size(&self) -> usize {
        assert!(!self.c_ptr.ptr().is_null());
        let sz = unsafe { c::packet_getTotalSize(self.c_ptr.ptr()) };
        sz as usize
    }

    pub fn header_size(&self) -> usize {
        assert!(!self.c_ptr.ptr().is_null());
        let sz = unsafe { c::packet_getHeaderSize(self.c_ptr.ptr()) };
        sz as usize
    }

    pub fn payload_size(&self) -> usize {
        assert!(!self.c_ptr.ptr().is_null());
        let sz = unsafe { c::packet_getPayloadSize(self.c_ptr.ptr()) };
        sz as usize
    }

    pub fn add_status(&mut self, status: PacketStatus) {
        assert!(!self.c_ptr.ptr().is_null());
        let status_flag = status as c::PacketDeliveryStatusFlags;
        unsafe { c::packet_addDeliveryStatus(self.c_ptr.ptr(), status_flag) };
    }

    pub fn src_address(&self) -> SocketAddrV4 {
        let ip = Ipv4Addr::from(u32::from_be(unsafe {
            c::packet_getSourceIP(self.c_ptr.ptr())
        }));
        let port = u16::from_be(unsafe { c::packet_getSourcePort(self.c_ptr.ptr()) });

        SocketAddrV4::new(ip, port)
    }

    pub fn dst_address(&self) -> SocketAddrV4 {
        let ip = Ipv4Addr::from(u32::from_be(unsafe {
            c::packet_getDestinationIP(self.c_ptr.ptr())
        }));
        let port = u16::from_be(unsafe { c::packet_getDestinationPort(self.c_ptr.ptr()) });

        SocketAddrV4::new(ip, port)
    }

    pub fn priority(&self) -> FifoPacketPriority {
        unsafe { c::packet_getPriority(self.c_ptr.ptr()) }
    }

    /// Transfers ownership of the given c_ptr reference into a new rust packet
    /// object.
    pub fn from_raw(c_ptr: *mut c::Packet) -> Self {
        assert!(!c_ptr.is_null());
        Self {
            c_ptr: unsafe { SyncSendPointer::new(c_ptr) },
        }
    }

    /// Transfers ownership of the inner c_ptr reference to the caller while
    /// dropping the rust packet object.
    pub fn into_inner(mut self) -> *mut c::Packet {
        // We want to keep the c ref when the rust packet is dropped.
        let c_ptr = self.c_ptr.ptr();
        self.c_ptr = unsafe { SyncSendPointer::new(std::ptr::null_mut()) };
        c_ptr
    }

    pub fn borrow_inner(&self) -> *mut c::Packet {
        self.c_ptr.ptr()
    }
}

impl Drop for PacketRc {
    fn drop(&mut self) {
        if !self.c_ptr.ptr().is_null() {
            // If the rust packet is dropped before into_inner() is called,
            // we also drop the c packet ref to free it.
            unsafe { c::packet_unref(self.c_ptr.ptr()) }
        }
    }
}

impl PacketDisplay for PacketRc {
    fn display_bytes(&self, writer: impl Write) -> std::io::Result<()> {
        self.borrow_inner().cast_const().display_bytes(writer)
    }
}

impl PacketDisplay for *const c::Packet {
    fn display_bytes(&self, mut writer: impl Write) -> std::io::Result<()> {
        assert!(!self.is_null());

        let header_len: u16 = unsafe { c::packet_getHeaderSize(*self) }
            .try_into()
            .unwrap();
        let payload_len: u16 = unsafe { c::packet_getPayloadSize(*self) }
            .try_into()
            .unwrap();
        let protocol = unsafe { c::packet_getProtocol(*self) };

        // write the IP header

        let version_and_header_length: u8 = 0x45;
        let fields: u8 = 0x0;
        let total_length: u16 = header_len + payload_len;
        let identification: u16 = 0x0;
        let flags_and_fragment: u16 = 0x4000;
        let time_to_live: u8 = 64;
        let iana_protocol: u8 = match protocol {
            c::_ProtocolType_PTCP => 6,
            c::_ProtocolType_PUDP => 17,
            _ => panic!("Unexpected packet protocol"),
        };
        let header_checksum: u16 = 0x0;
        let source_ip: [u8; 4] =
            u32::from_be(unsafe { c::packet_getSourceIP(*self) }).to_be_bytes();
        let dest_ip: [u8; 4] =
            u32::from_be(unsafe { c::packet_getDestinationIP(*self) }).to_be_bytes();

        // version and header length: 1 byte
        // DSCP + ECN: 1 byte
        writer.write_all(&[version_and_header_length, fields])?;
        // total length: 2 bytes
        writer.write_all(&total_length.to_be_bytes())?;
        // identification: 2 bytes
        writer.write_all(&identification.to_be_bytes())?;
        // flags + fragment offset: 2 bytes
        writer.write_all(&flags_and_fragment.to_be_bytes())?;
        // ttl: 1 byte
        // protocol: 1 byte
        writer.write_all(&[time_to_live, iana_protocol])?;
        // header checksum: 2 bytes
        writer.write_all(&header_checksum.to_be_bytes())?;
        // source IP: 4 bytes
        writer.write_all(&source_ip)?;
        // destination IP: 4 bytes
        writer.write_all(&dest_ip)?;

        // write protocol-specific data

        match protocol {
            c::_ProtocolType_PTCP => display_tcp_bytes(*self, &mut writer)?,
            c::_ProtocolType_PUDP => display_udp_bytes(*self, &mut writer)?,
            _ => panic!("Unexpected packet protocol"),
        }

        // write payload data

        if payload_len > 0 {
            // shadow's packet payloads are guarded by a mutex, so it's easiest to make a copy of them
            let mut payload_buf = vec![0u8; payload_len.into()];
            let count = unsafe {
                c::packet_copyPayloadShadow(
                    *self,
                    0,
                    payload_buf.as_mut_ptr() as *mut libc::c_void,
                    payload_len.into(),
                )
            };
            assert_eq!(
                count,
                u32::from(payload_len),
                "Packet payload somehow changed size"
            );

            // packet payload: `payload_len` bytes
            writer.write_all(&payload_buf)?;
        }

        Ok(())
    }
}

/// Helper for writing the tcp bytes of the packet.
fn display_tcp_bytes(packet: *const c::Packet, mut writer: impl Write) -> std::io::Result<()> {
    assert_eq!(
        unsafe { c::packet_getProtocol(packet) },
        c::_ProtocolType_PTCP
    );

    let tcp_header = unsafe { c::packet_getTCPHeader(packet) };
    assert!(!tcp_header.is_null());
    assert_eq!(
        tcp_header as usize % std::mem::align_of::<c::PacketTCPHeader>(),
        0
    );
    let tcp_header = unsafe { tcp_header.as_ref() }.unwrap();

    // process TCP options

    let window_scale = tcp_header.windowScaleSet.then_some(tcp_header.windowScale);

    // options can be a max of 40 bytes
    let mut options = [0u8; 40];
    let mut options_len = 0;

    if let Some(window_scale) = window_scale {
        // option-kind = 3, option-len = 3, option-data = window-scale
        options[options_len..][..3].copy_from_slice(&[3, 3, window_scale]);
        options_len += 3;
    }

    if options_len % 4 != 0 {
        // need to add padding (our options array was already initialized with zeroes)
        let padding = 4 - (options_len % 4);
        options_len += padding;
    }

    let options = &options[..options_len];

    // write the TCP header

    let source_port: [u8; 2] =
        u16::from_be(unsafe { c::packet_getSourcePort(packet) }).to_be_bytes();
    let dest_port: [u8; 2] =
        u16::from_be(unsafe { c::packet_getDestinationPort(packet) }).to_be_bytes();
    let sequence: [u8; 4] = tcp_header.sequence.to_be_bytes();
    let ack: [u8; 4] = if tcp_header.flags & c::ProtocolTCPFlags_PTCP_ACK != 0 {
        tcp_header.acknowledgment.to_be_bytes()
    } else {
        0u32.to_be_bytes()
    };

    // c::CONFIG_HEADER_SIZE is in bytes. Ultimately, TCP header len is represented in 32-bit
    // words, so we divide by 4. The left-shift of 4 is because the header len is represented
    // in the top 4 bits.
    let mut header_len: u8 = c::CONFIG_HEADER_SIZE_TCP.try_into().unwrap();
    header_len += u8::try_from(options.len()).unwrap();
    header_len /= 4;
    header_len <<= 4;

    let mut tcp_flags: u8 = 0;
    if tcp_header.flags & c::ProtocolTCPFlags_PTCP_RST != 0 {
        tcp_flags |= 0x04;
    }
    if tcp_header.flags & c::ProtocolTCPFlags_PTCP_SYN != 0 {
        tcp_flags |= 0x02;
    }
    if tcp_header.flags & c::ProtocolTCPFlags_PTCP_ACK != 0 {
        tcp_flags |= 0x10;
    }
    if tcp_header.flags & c::ProtocolTCPFlags_PTCP_FIN != 0 {
        tcp_flags |= 0x01;
    }
    let window: [u8; 2] = u16::try_from(tcp_header.window).unwrap().to_be_bytes();
    let checksum: u16 = 0x0;
    let urgent_pointer: u16 = 0x0;

    // source port: 2 bytes
    writer.write_all(&source_port)?;
    // destination port: 2 bytes
    writer.write_all(&dest_port)?;
    // sequence number: 4 bytes
    writer.write_all(&sequence)?;
    // acknowledgement number: 4 bytes
    writer.write_all(&ack)?;
    // data offset + reserved + NS: 1 byte
    // flags: 1 byte
    writer.write_all(&[header_len, tcp_flags])?;
    // window size: 2 bytes
    writer.write_all(&window)?;
    // checksum: 2 bytes
    writer.write_all(&checksum.to_be_bytes())?;

    writer.write_all(&urgent_pointer.to_be_bytes())?;

    writer.write_all(options)?;

    Ok(())
}

/// Helper for writing the udp bytes of the packet.
fn display_udp_bytes(packet: *const c::Packet, mut writer: impl Write) -> std::io::Result<()> {
    assert_eq!(
        unsafe { c::packet_getProtocol(packet) },
        c::_ProtocolType_PUDP
    );

    // write the UDP header

    let source_port: [u8; 2] =
        u16::from_be(unsafe { c::packet_getSourcePort(packet) }).to_be_bytes();
    let dest_port: [u8; 2] =
        u16::from_be(unsafe { c::packet_getDestinationPort(packet) }).to_be_bytes();
    let udp_len: u16 = u16::try_from(unsafe { c::packet_getPayloadSize(packet) })
        .unwrap()
        .checked_add(8)
        .unwrap();
    let checksum: u16 = 0x0;

    // source port: 2 bytes
    writer.write_all(&source_port)?;
    // destination port: 2 bytes
    writer.write_all(&dest_port)?;
    // length: 2 bytes
    writer.write_all(&udp_len.to_be_bytes())?;
    // checksum: 2 bytes
    writer.write_all(&checksum.to_be_bytes())?;

    Ok(())
}

pub fn to_legacy_tcp_flags(flags: tcp::TcpFlags) -> c::ProtocolTCPFlags {
    let mut new_flags = c::ProtocolTCPFlags_PTCP_NONE;

    for flag in flags.iter() {
        match flag {
            tcp::TcpFlags::FIN => new_flags |= c::ProtocolTCPFlags_PTCP_FIN,
            tcp::TcpFlags::SYN => new_flags |= c::ProtocolTCPFlags_PTCP_SYN,
            tcp::TcpFlags::RST => new_flags |= c::ProtocolTCPFlags_PTCP_RST,
            tcp::TcpFlags::PSH => panic!("Unsupported TCP flag: {flag:?}"),
            tcp::TcpFlags::ACK => new_flags |= c::ProtocolTCPFlags_PTCP_ACK,
            tcp::TcpFlags::URG => panic!("Unsupported TCP flag: {flag:?}"),
            tcp::TcpFlags::ECE => panic!("Unsupported TCP flag: {flag:?}"),
            tcp::TcpFlags::CWR => panic!("Unsupported TCP flag: {flag:?}"),
            _ => unreachable!(
                "Each bit is covered by a flag, so the iterator either returned multiple flags at \
                once or no flags: {flag:?}"
            ),
        }
    }

    new_flags
}

pub fn from_legacy_tcp_flags(mut flags: c::ProtocolTCPFlags) -> tcp::TcpFlags {
    let mut new_flags = tcp::TcpFlags::empty();

    if flags & c::ProtocolTCPFlags_PTCP_RST != 0 {
        new_flags.insert(tcp::TcpFlags::RST);
        flags &= !c::ProtocolTCPFlags_PTCP_RST;
    }

    if flags & c::ProtocolTCPFlags_PTCP_SYN != 0 {
        new_flags.insert(tcp::TcpFlags::SYN);
        flags &= !c::ProtocolTCPFlags_PTCP_SYN;
    }

    if flags & c::ProtocolTCPFlags_PTCP_ACK != 0 {
        new_flags.insert(tcp::TcpFlags::ACK);
        flags &= !c::ProtocolTCPFlags_PTCP_ACK;
    }

    if flags & c::ProtocolTCPFlags_PTCP_FIN != 0 {
        new_flags.insert(tcp::TcpFlags::FIN);
        flags &= !c::ProtocolTCPFlags_PTCP_FIN;
    }

    assert_eq!(flags, c::ProtocolTCPFlags_PTCP_NONE, "Unexpected TCP flags");

    new_flags
}
