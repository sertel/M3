
use crate::errors::Error;
use crate::com::{RecvGate, SendGate, SGateArgs};
use crate::com::stream::{recv_msg};
use crate::serialize::{Serialize, Deserialize};
use crate::cap::Selector;
use crate::math;
use crate::tiles::{Activity, ChildActivity};


pub struct Sender {
    sgate: SendGate
}

pub struct Receiver {
    rgate: RecvGate
}

pub trait Capable {
    fn sel(&self) -> Selector;
}

impl Sender{
    fn new(rgate: &RecvGate, credits: u32) -> Result<Self, Error> {
        let sgate = SendGate::new_with(SGateArgs::new(rgate).credits(credits))?;
        Ok(Sender { sgate })
    }

    pub fn activate() -> Result<Self, Error> {
        let mut target = Activity::own().data_source();
        let sgate = SendGate::new_bind(target.pop()?);
        sgate.activate()?;
        Ok(Sender { sgate })
    }

    pub fn send<T: Serialize>(&self, data: T) -> Result<(), Error> {
        send_vmsg!(&self.sgate, RecvGate::def(), data)
    }
}

impl Capable for Sender {
    fn sel(&self) -> Selector {
        self.sgate.sel()
    }
}

impl Receiver {
    pub fn new(order: usize, msg_order: usize) -> Result<Self, Error> {
        let rgate = RecvGate::new(math::next_log2(order), math::next_log2(msg_order))?; 
        Ok(Receiver { rgate } )
    }

    pub fn activate(order: usize, msg_order: usize) -> Result<Self, Error> {
        let mut src = Activity::own().data_source();
        let mut rgate = RecvGate::new_bind(
            src.pop()?,
            math::next_log2(order), 
            math::next_log2(msg_order)); 
        rgate.activate()?;
        Ok(Receiver { rgate } )
    }

    pub fn activate_def() -> Result<Self, Error> {
        Self::activate(256, 256)
    }

    pub fn cap_sel(&self) -> Selector {
        self.rgate.sel()
    }

    fn sender(&self, credits: u32) -> Result<Sender, Error> {
        Sender::new(&self.rgate, credits)
    }

    pub fn recv<T: Deserialize<'static>>(&self) -> Result<T,Error> {
        recv_msg(&self.rgate)?.pop::<T>()
    }
}

impl Capable for Receiver {
    fn sel(&self) -> Selector {
        self.rgate.sel()
    }
}

pub fn channel(order: usize, msg_order: usize, credits: u32) -> Result<(Sender, Receiver), Error> {
    let rx = Receiver::new(order, msg_order)?;
    let tx = rx.sender(credits)?;
    Ok((tx, rx))
}

pub fn channel_def() -> Result<(Sender, Receiver), Error> {
    channel(256, 256, 1)
}

pub trait Register: Capable {
    fn register(&self, act: &mut ChildActivity) -> Result<(), Error>{
        // pass capabilty to activity
        act.delegate_obj(self.sel())?;
    
        // pass selector to activity
        let mut act_sels = act.data_sink();
        act_sels.push(self.sel());
        Ok(())
    }
}

impl Register for Sender {}
impl Register for Receiver {}
