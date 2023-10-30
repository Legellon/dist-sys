pub mod node;

use anyhow::{anyhow, Context};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

pub trait NodeBuilder<I: DeserializeOwned, N: Node<I>> {
    fn to_node(&self) -> anyhow::Result<N>;
}

impl<I: DeserializeOwned, N> NodeBuilder<I, N> for Message<'_, I, InitPayload>
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

pub trait Node<MI: DeserializeOwned>: IdGen<MI> + Default {
    type Init;
    type Payload;

    fn step(&mut self, msg: Message<MI, Self::Payload>, out: &mut impl Write)
        -> anyhow::Result<()>;

    fn init(&mut self, init: Self::Init);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<'a, Id, Payload> {
    #[serde(rename = "msg_id")]
    id: Option<Id>,
    #[serde(bound(deserialize = "&'a Id: Deserialize<'a>"))]
    in_reply_to: Option<&'a Id>,
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<'a, Id, Payload> {
    src: &'a str,
    #[serde(rename = "dest")]
    dst: &'a str,
    body: Body<'a, Id, Payload>,
    #[serde(skip)]
    linked: bool,
}

pub trait PayloadLinker<Id, Payload> {
    type Output<'a>
    where
        Self: 'a;

    fn link(&self, id: Id, payload: Payload) -> Self::Output<'_>;
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

impl<I: DeserializeOwned, OP, IP> PayloadLinker<I, OP> for Message<'_, I, IP> {
    type Output<'a> = Message<'a, I, OP> where Self: 'a;

    fn link(&self, id: I, payload: OP) -> Self::Output<'_> {
        Message {
            src: self.dst,
            dst: self.src,
            body: Body {
                id: Some(id),
                in_reply_to: self.body.id.as_ref(),
                payload,
            },
            linked: true,
        }
    }
}

impl<I, P> Message<'_, I, P>
where
    I: DeserializeOwned + Serialize,
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
    N::Payload: DeserializeOwned,
    Id: DeserializeOwned + Serialize + AsRef<Id> + LocalUniq,
{
    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();

    let mut stdout = std::io::stdout().lock();

    // We must to create a binding variable,
    // because we need to stick a ref to specific lifetime.
    let init_str = &stdin
        .next()
        .expect("at least one message should be received (init)")?;
    let init_msg: Message<Id, InitPayload> =
        serde_json::from_str(init_str).context("failed to deserialize init message")?;

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
