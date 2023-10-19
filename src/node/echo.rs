use crate::{IdGen, LocalUniq, Message, Node, NodeBase, PayloadLinker};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::io::Write;

pub struct EchoNode<Init, Id: Default>
where
    Id: LocalUniq,
{
    base: NodeBase<Init, Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum EchoPayload {
    Echo { echo: String },
    EchoOk { echo: String },
}

impl<Init: Default, Id: Default + LocalUniq> Default for EchoNode<Init, Id> {
    fn default() -> Self {
        Self {
            base: NodeBase::default(),
        }
    }
}

impl<Init, Id: Default + LocalUniq> IdGen<Id> for EchoNode<Init, Id> {
    fn gen_id(&mut self) -> Id {
        self.base.gen_id()
    }
}

impl<T, I> Node<I> for EchoNode<T, I>
where
    I: Clone + Serialize + LocalUniq + Default,
    T: Default,
{
    type Init = T;
    type Payload = EchoPayload;

    fn step(&mut self, msg: Message<I, Self::Payload>, out: &mut impl Write) -> anyhow::Result<()> {
        match msg.body.payload {
            EchoPayload::Echo { ref echo } => {
                let echo = echo.clone();
                msg.link(self.gen_id(), EchoPayload::EchoOk { echo })
                    .send(out)
                    .context("send echo_ok reply")?;
            }
            _ => {}
        }
        Ok(())
    }

    fn init(&mut self, init: Self::Init) {
        self.base.init(init);
    }
}
