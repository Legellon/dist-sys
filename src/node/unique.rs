use crate::{IdGen, Init, LocalUniq, Message, Node, NodeBase, PayloadLinker};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub struct UniqueNode<Init, Id: Default>
where
    Id: LocalUniq,
{
    base: NodeBase<Init, Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum UniquePayload<I> {
    Generate,
    GenerateOk { id: I },
}

impl<Init: Default, Id: Default> Default for UniqueNode<Init, Id>
where
    Id: LocalUniq,
{
    fn default() -> Self {
        Self {
            base: NodeBase::default(),
        }
    }
}

impl<Init, Id: Default> IdGen<Id> for UniqueNode<Init, Id>
where
    Id: LocalUniq,
{
    fn gen_id(&mut self) -> Id {
        self.base.gen_id()
    }
}

impl<I: Default> Node<I> for UniqueNode<Init, I>
where
    I: LocalUniq + Serialize + Clone,
{
    type Init = Init;
    type Payload = UniquePayload<String>;

    fn step(
        &mut self,
        msg: Message<I, Self::Payload>,
        out: &mut impl std::io::Write,
    ) -> anyhow::Result<()> {
        match msg.body.payload {
            UniquePayload::Generate => {
                let id = format!(
                    "{}{}",
                    self.base
                        .init
                        .clone()
                        .ok_or(anyhow!("to generate uniq id, init state must be set"))?
                        .node_id,
                    self.base.next_id
                );
                msg.link(self.gen_id(), UniquePayload::GenerateOk { id })
                    .send(out)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn init(&mut self, init: Self::Init) {
        self.base.init(init);
    }
}
