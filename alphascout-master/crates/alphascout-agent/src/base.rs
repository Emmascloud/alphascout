use ethers::providers::{
    Provider,
    Http,
};

pub fn base_provider()
-> Provider<Http> {

    let rpc =
        std::env::var("BASE_RPC_URL")
            .unwrap();

    Provider::<Http>::try_from(rpc)
        .unwrap()
}
