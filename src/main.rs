use bitwarden::auth::login::PasswordLoginRequest;
use bitwarden::platform::SyncRequest;
use bitwarden::Client;
use clap::Parser;
use fuser::Filesystem;

#[derive(Debug, Parser)]
struct Args {
    #[clap()]
    mountpoint: String,
}

struct NullFS;

impl Filesystem for NullFS {}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{:?}", args);

    let mut bw_client = Client::new(None);
    let email = "andrewjeffery97@gmail.com".to_owned();

    let kdf = bw_client.auth().prelogin(email.clone()).await.unwrap();
    println!("Got kdf from prelogin: {:?}", kdf);

    let password = rpassword::prompt_password("Password: ").unwrap();
    println!("Got password, logging in");
    let bw_password = PasswordLoginRequest {
        email,
        password,
        two_factor: None,
        kdf,
    };
    let login_res = bw_client.auth().login_password(&bw_password).await;

    match login_res {
        Ok(response) => {
            println!("Logged in! {:?}", response);
            let organization_id = uuid::Uuid::nil();
            println!("Renewing token");
            bw_client.auth().renew_token().await.unwrap();
            println!("syncing");
            let sync_res = bw_client
                .sync(&SyncRequest {
                    exclude_subdomains: None,
                })
                .await
                .unwrap();
            println!("synced! {:?}", sync_res);
            // println!("Listing secrets");
            // let secrets = bw_client.secrets().list(
            //     &bitwarden::secrets_manager::secrets::SecretIdentifiersRequest {
            //         organization_id,
            //     },
            // ).await.unwrap();
            // for secret in secrets.data {
            //     println!("{secret:?}")
            // }
            // println!("Configuring mount");
            // fuser::mount2(NullFS, args.mountpoint, &[]).unwrap();
        }
        Err(err) => {
            println!("Failed to log in: {}", err);
        }
    }
}
