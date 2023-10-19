use dist_sys::{main_loop, node::UniqueNode, Init};

type MsgId = usize;
type NodeInst = UniqueNode<Init, MsgId>;

fn main() -> anyhow::Result<()> {
    main_loop::<MsgId, NodeInst>()
}
