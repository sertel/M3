use cap::Selector;
use com::*;
use errors::Error;
use session::Session;

int_enum! {
    struct MetaOp : u32 {
        const ATTACH      = 0x0;
        const CLOSE       = 0x1;
    }
}

pub struct Pipe {
    sess: Session,
    meta_gate: SendGate,
    rd_gate: SendGate,
    wr_gate: SendGate,
}

impl Pipe {
    pub fn new(name: &str, mem_size: usize) -> Result<Self, Error> {
        let sess = Session::new(name, mem_size as u64)?;
        let meta = SendGate::new_bind(sess.obtain_obj()?);
        let read = SendGate::new_bind(sess.obtain_obj()?);
        let write = SendGate::new_bind(sess.obtain_obj()?);
        Ok(Pipe {
            sess: sess,
            meta_gate: meta,
            rd_gate: read,
            wr_gate: write,
        })
    }

    pub fn new_bind(sess: Selector, meta_gate: Selector, rd_gate: Selector, wr_gate: Selector) -> Self {
        Pipe {
            sess: Session::new_bind(sess),
            meta_gate: SendGate::new_bind(meta_gate),
            rd_gate: SendGate::new_bind(rd_gate),
            wr_gate: SendGate::new_bind(wr_gate),
        }
    }

    pub fn sel(&self) -> Selector {
        self.sess.sel()
    }
    pub fn meta_gate_sel(&self) -> Selector {
        self.meta_gate.sel()
    }
    pub fn read_gate_sel(&self) -> Selector {
        self.rd_gate.sel()
    }
    pub fn write_gate_sel(&self) -> Selector {
        self.wr_gate.sel()
    }

    pub fn attach(&self, reading: bool) -> Result<(), Error> {
        send_recv_res!(
            &self.meta_gate, RecvGate::def(),
            MetaOp::ATTACH, reading as u8
        ).map(|_| ())
    }

    pub fn request_read(&self, amount: usize) -> Result<(usize, usize), Error> {
        let mut reply = send_recv_res!(
            &self.rd_gate, RecvGate::def(),
            amount
        )?;
        Ok((reply.pop(), reply.pop()))
    }

    pub fn request_write(&self, amount: usize, last_write: usize) -> Result<usize, Error> {
        let mut reply = send_recv_res!(
            &self.wr_gate, RecvGate::def(),
            amount, last_write
        )?;
        Ok(reply.pop())
    }

    pub fn close(&self, reading: bool, last_write: usize) -> Result<(), Error> {
        send_recv_res!(
            &self.meta_gate, RecvGate::def(),
            MetaOp::CLOSE, reading as u8, last_write
        ).map(|_| ())
    }
}
