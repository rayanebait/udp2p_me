use std::future::IntoFuture;
use std::io::Error;
use std::str::FromStr;

use tokio::time::{Duration, sleep};
use tokio::{join, try_join, select};
use tokio_macros;
use reqwest::{Url,Client};
use lib_web::discovery::{*};

async fn func1(dur: u64, s: impl Into<String>)->bool{
    let sl = sleep(Duration::from_millis(dur));
    if sl.is_elapsed() {
        println!("{} done!", s.into());
        true
    } else {
        println!("{}", s.into());
        false
    }
}

async fn func2(){
    let mut i=0;
    let fut = func1(2, format!("tour {}", 10));
    let x = loop{
        i+=1;
    };

    join!(fut);
}

#[tokio::main]
async fn main() {
    loop{
        let client = Client::builder()
                        .connect_timeout(Duration::from_secs(3))
                        .user_agent("me")
                        .build()
                        .unwrap();
        let url1 = Url::from_str("https://jch.irif.fr:8443/peers/").unwrap();
        let url2 = Url::from_str("https://jch.irif.fr:8443/peers/jch.irif.fr/addresses").unwrap();
        let url3 = Url::from_str("https://jch.irif.fr:8443/peers/jch.irif.fr/root").unwrap();

        let mut fresponse1 = client.get(url1).send();
        let mut fresponse2 = client.get(url2).send();
        let mut fresponse3 = client.get(url3).send();

        select! {
            resp1 = (&mut fresponse1) =>{
                let r1 = resp1.unwrap().text().await.unwrap();
                println!("{}", r1);
            }
            resp2 = (&mut fresponse2) =>{
                let r2 = resp2.unwrap().text().await.unwrap();
                println!("{}", r2);
            }
            resp3 = (&mut fresponse3) =>{
                let r3 = resp3.unwrap().text().await.unwrap();
                println!("{:02x?}", r3.as_bytes());
            }
        }
        sleep(Duration::from_secs(1));
    }


}
