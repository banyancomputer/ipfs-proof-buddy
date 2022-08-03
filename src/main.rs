mod proof_gen;
mod deal_tracker;

// want to be able to accept a file from estuary, stick it in ipfs, keep it in a database with proof info, submit proofs regularly, and close out of deals.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open a connection to the mini-redis address.
    //let mut client = client::connect("127.0.0.1:6379").await?;

    // Set the key "hello" with value "world"
    //client.set("hello", "world".into()).await?;

    // Get key "hello"
    //let result = client.get("hello").await?;

    println!("got value from the server; result={:?}", "bloobloobloo");

    Ok(())
}
