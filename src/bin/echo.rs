use dist_sys::{main_loop, node::EchoNode, Init};

type MsgId = usize;
type NodeInst = EchoNode<Init, MsgId>;

fn main() -> anyhow::Result<()> {
    main_loop::<MsgId, NodeInst>()
}
