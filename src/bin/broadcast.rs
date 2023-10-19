use dist_sys::{main_loop, node::BroadcastNode, Init};

type MsgId = usize;
type MsgContent = usize;
type NodeInst = BroadcastNode<Init, MsgId, MsgContent>;

fn main() -> anyhow::Result<()> {
    main_loop::<MsgId, NodeInst>()
}
