use alloc::{vec::Vec, sync::Arc};
use futures::channel::mpsc;
use spin::Mutex;


/// A mpmc channel where **ALL** receiver receives the sent message (i.e. a broadcast channel). 
#[derive(Clone)]
pub struct Sender {
    senders: Arc<Mutex<Vec<mpsc::Sender<u8>>>>,
}

type Receiver = mpsc::Receiver<u8>;

impl Sender {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn send_status(&mut self, status: u8) {
        let mut senders = self.senders.lock();
        senders.retain_mut(|sender| {
            let _ = sender.try_send(status);
            // Delete from senders if closed
            !sender.is_closed()
        });
    }

    pub fn get_new_receiver(&mut self) -> Receiver {
        let mut senders = self.senders.lock();
        let (sender, receiver) = mpsc::channel(1);
        senders.push(sender);
        receiver
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_sender() {
        let mut sender = Sender::new();
        let mut sender1 = sender.clone();
        let mut receiver1 = sender1.get_new_receiver();
        let mut receiver2 = sender1.get_new_receiver();

        sender.send_status(1);

        assert_eq!(receiver1.try_next().unwrap(), Some(1));
        assert!(receiver1.try_next().is_err());

        assert_eq!(receiver2.try_next().unwrap(), Some(1));
        assert!(receiver2.try_next().is_err());

    }
}