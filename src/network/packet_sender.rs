
use std::io;
use std::time::Instant;
use std::sync::Arc;
use crossbeam::sync::MsQueue;
use mio::{Ready, Poll, PollOpt, Token, Registration, SetReadiness, Evented};
use super::packet::GamePacket;

pub struct PacketSender {
    pub outbound: Arc<MsQueue<GamePacket>>,
    registration: Registration,
    set_readiness: SetReadiness,
}
impl PacketSender {
    pub fn new() -> PacketSender {
        let (registration, set_readiness) = Registration::new2();
        PacketSender {
            outbound: Arc::new(MsQueue::new()),
            registration: registration,
            set_readiness: set_readiness
        }
    }

    pub fn send(&self, packet: GamePacket) -> ::errors::Result<()>
    {
        self.outbound.push(packet);
        self.set_readiness.set_readiness(Ready::readable())?;
        Ok(())
    }

    pub fn send_at_future_time(&self, packet: GamePacket, when: Instant) -> ::errors::Result<()>
    {
        let msqueue = self.outbound.clone();
        let setr = self.set_readiness.clone();
        ::std::thread::spawn(move|| {
            let now = Instant::now();
            if now < when {
                ::std::thread::sleep(when - now);
            }
            msqueue.push(packet);
            let _ = setr.set_readiness(Ready::readable());
        });
        Ok(())
    }
}

impl Evented for PacketSender {
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
                -> io::Result<()>
    {
        self.registration.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
        -> io::Result<()>
    {
        self.registration.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        // For some reason, rust is choosing the wrong fn. Once it goes away entirely
        // this warning will disappear.
        #[allow(deprecated)]
        self.registration.deregister(poll)
    }
}
