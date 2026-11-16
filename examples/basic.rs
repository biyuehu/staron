use staron::{Block, StaronOptions, gate};

#[tokio::main]
async fn main() {
  gate(
    StaronOptions::new("Ov23liCJaZwQGcVvHs9X", "biyuehu", "staron")
      .on_fail(Block(Some(
        "Please star biyuehu/staron on GitHub to use this tool.",
      )))
      .on_success(|ctx| async move {
        println!("success: {ctx:?}");
      })
      .skip_star_for_owner(true),
  )
  .await;
}
