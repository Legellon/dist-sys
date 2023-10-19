pub mod node;

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    io::{BufRead, Write},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Init {
    node_id: String,
    node_ids: Vec<String>,
}

#[derive(Default)]
pub struct NodeBase<Init, Id: LocalUniq + Default> {
    next_id: Id,
    init: Option<Init>,
}

impl<Init, Id: LocalUniq + Default> NodeBase<Init, Id> {
    fn init(&mut self, init: Init) {
        self.init = Some(init);
    }
}

pub trait NodeBuilder<I: Clone, N: Node<I>> {
    fn to_node(&self) -> anyhow::Result<N>;
}

impl<I: Clone, N> NodeBuilder<I, N> for Message<I, InitPayload>
where
    N: Node<I, Init = Init>,
{
    fn to_node(&self) -> anyhow::Result<N> {
        let mut node = N::default();
        let init = match &self.body.payload {
            InitPayload::Init(init) => init.clone(),
            InitPayload::InitOk => return Err(anyhow!("impossible to build a node from init_ok")),
        };
        node.init(init);
        Ok(node)
    }
}

pub trait Node<MI: Clone>: IdGen<MI> + Default {
    type Init;
    type Payload;

    fn step(&mut self, msg: Message<MI, Self::Payload>, out: &mut impl Write)
        -> anyhow::Result<()>;

    fn init(&mut self, init: Self::Init);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Id: Clone, Payload> {
    #[serde(rename = "msg_id")]
    id: Option<Id>,
    in_reply_to: Option<Id>,
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Id: Clone, Payload> {
    src: String,
    #[serde(rename = "dest")]
    dst: String,
    body: Body<Id, Payload>,
    #[serde(skip)]
    linked: bool,
}

pub trait PayloadLinker<Id, Payload> {
    type Output;

    fn link(&self, id: Id, payload: Payload) -> Self::Output;
}

pub trait LocalUniq: Display {
    fn begin() -> Self;
    fn reserve(&mut self) -> Self;
}

pub trait IdGen<Id> {
    fn gen_id(&mut self) -> Id;
}

impl<T: LocalUniq + Default, Y> IdGen<T> for NodeBase<Y, T> {
    fn gen_id(&mut self) -> T {
        self.next_id.reserve()
    }
}

impl LocalUniq for usize {
    fn reserve(&mut self) -> Self {
        let n = *self;
        *self += 1;
        n
    }

    fn begin() -> Self {
        usize::MIN
    }
}

impl<I: Clone, OP, IP> PayloadLinker<I, OP> for Message<I, IP> {
    type Output = Message<I, OP>;

    fn link(&self, id: I, payload: OP) -> Self::Output {
        Message {
            src: self.dst.clone(),
            dst: self.src.clone(),
            body: Body {
                id: Some(id),
                in_reply_to: self.body.id.clone(),
                payload,
            },
            linked: true,
        }
    }
}

impl<I, P> Message<I, P>
where
    I: Clone + Serialize,
    P: Serialize,
{
    fn send(&self, out: &mut impl Write) -> anyhow::Result<()> {
        if self.linked {
            serde_json::to_writer(&mut *out, self).context("serialize reply message")?;
            out.write_all(b"\n").context("write trailing newline")?;
            return Ok(());
        }
        Err(anyhow!("msg must be linked to send it"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InitPayload {
    Init(Init),
    InitOk,
}

pub fn main_loop<Id, N: Node<Id, Init = Init>>() -> anyhow::Result<()>
where
    N::Payload: for<'a> Deserialize<'a>,
    Id: for<'a> Deserialize<'a> + Serialize + Clone + LocalUniq,
{
    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();

    let mut stdout = std::io::stdout().lock();

    let init_msg: Message<Id, InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("at least one message should be received (init)")?,
    )
    .context("failed to deserialize init message")?;

    let mut node: N = init_msg.to_node()?;
    init_msg
        .link(node.gen_id(), InitPayload::InitOk)
        .send(&mut stdout)?;

    for line in stdin {
        let line = line.context("failed to deserialize input from STDIN")?;
        let msg: Message<Id, N::Payload> = serde_json::from_str(&line)?;
        node.step(msg, &mut stdout)?;
    }

    Ok(())
}
