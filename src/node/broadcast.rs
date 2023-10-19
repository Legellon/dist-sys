use crate::{IdGen, LocalUniq, Message, Node, NodeBase, PayloadLinker};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Topology {
    #[serde(rename = "topology")]
    tree: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum BroadcastPayload<T: Eq> {
    Broadcast { message: T },
    BroadcastOk,
    Read,
    ReadOk { messages: Vec<T> },
    Topology(Topology),
    TopologyOk,
}

#[derive(Default)]
pub struct BroadcastNode<Init, Id, M>
where
    Init: Default,
    Id: Clone + Default + LocalUniq,
    M: Eq + Serialize,
{
    base: NodeBase<Init, Id>,
    // Because we will store relatively simple types to compare,
    // that's may be more efficient to use Vec instead of HashSet
    // due to the CPU cache (looks good in theory, but this likely is wrong).
    messages: Vec<M>,
    topology: Topology,
}

impl<T, I, M> IdGen<I> for BroadcastNode<T, I, M>
where
    T: Default,
    I: Clone + Default + LocalUniq,
    M: Eq + Default + Serialize,
{
    fn gen_id(&mut self) -> I {
        self.base.gen_id()
    }
}

impl<Init, Id, M> Node<Id> for BroadcastNode<Init, Id, M>
where
    Init: Default,
    Id: Clone + Default + Serialize + LocalUniq,
    M: Clone + Eq + Default + Serialize,
{
    type Init = Init;
    type Payload = BroadcastPayload<M>;

    fn step(
        &mut self,
        msg: Message<Id, Self::Payload>,
        out: &mut impl Write,
    ) -> anyhow::Result<()> {
        match msg.body.payload {
            BroadcastPayload::Broadcast { ref message } => {
                let message = message.clone();
                if self.messages.iter().all(|x| *x != message) {
                    self.messages.push(message);
                }
                msg.link(self.gen_id(), Self::Payload::BroadcastOk)
                    .send(out)?;
            }

            BroadcastPayload::Read => msg
                .link(
                    self.gen_id(),
                    Self::Payload::ReadOk {
                        messages: self.messages.clone(),
                    },
                )
                .send(out)?,

            BroadcastPayload::Topology(ref topology) => {
                self.topology = topology.clone();
                msg.link(self.gen_id(), Self::Payload::TopologyOk)
                    .send(out)?;
            }

            _ => {}
        };
        Ok(())
    }

    fn init(&mut self, init: Self::Init) {
        self.base.init(init);
    }
}
