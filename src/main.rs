use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::str;
use toml::Value;

fn main() {
    // println!("{:?}", wg_genkey());
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let path = args[1].clone();
    let v = read_config(&path);
    v["clients"]
        .as_array()
        .expect("Expect list of clients")
        .into_iter()
        .for_each(|x| {
            let name = x["_name"].as_str().expect("Expect name").to_string();
            let cfg = generate_config(&name, &v);
            fs::write(format!("{}.conf", name), cfg.join("\n")).expect("Failed writing conf");
        });
}

#[allow(dead_code)]
fn wg_genkey() -> (String, String) {
    let proc = Command::new("wg")
        .arg("genkey")
        .output()
        .expect("Failed to execute process")
        .stdout;
    let priv_key = str::from_utf8(&proc).expect("").to_string();
    let mut proc = Command::new("wg")
        .arg("pubkey")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute process");

    if let Some(mut stdin) = proc.stdin.take() {
        stdin.write_all(priv_key.as_bytes()).expect(""); // drop would happen here }
    }
    let pub_key = proc.wait_with_output().expect("Failed to get stdout");
    return (
        priv_key,
        str::from_utf8(&pub_key.stdout).expect("").to_string(),
    );
}

fn read_config(path: &str) -> Value {
    let mut file = File::open(path).expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");
    let value: Value = contents.parse().unwrap();
    value
}

fn generate_config(name: &str, cfg: &Value) -> Vec<String> {
    let clients = cfg["clients"].as_array().unwrap();

    let interface = clients
        .iter()
        .find(|x| x["_name"].as_str().unwrap() == name)
        .unwrap()
        .as_table()
        .unwrap();

    let mut config = Vec::<String>::new();

    config.push("[Interface]".to_string());

    interface
        .iter()
        .filter(|(k, _v)| {
            !k.starts_with("_")
                && !vec!["Endpoint", "AllowedIPs", "PublicKey", "PersistentKeepalive"]
                    .contains(&&k[..])
        })
        .for_each(|(k, v)| {
            config.push(format!(
                "{} = {}",
                k,
                match &v {
                    v if v.is_str() => format!("{}", v.as_str().unwrap().to_string()),
                    v if v.is_integer() => v.as_integer().unwrap().to_string(),
                    v if v.is_bool() => v.as_bool().unwrap().to_string(),
                    _ => panic!("Unsupported type"),
                }
            ));
        });

    clients
        .iter()
        .filter(|x| x["_name"].as_str().unwrap() != name)
        .for_each(|x| {
            config.push("".to_string());
            let peer =
                generate_peer_config(interface, &x["_name"].as_str().unwrap().to_string(), &cfg);
            config.extend(peer);
        });

    config.push("".to_string());
    config
}

fn generate_peer_config(
    interface: &BTreeMap<String, Value>,
    name: &str,
    cfg: &Value,
) -> Vec<String> {
    let clients = cfg["clients"].as_array().unwrap();
    let peer = clients
        .iter()
        .find(|x| x["_name"].as_str().unwrap() == name)
        .unwrap()
        .as_table()
        .unwrap();
    let mut config = Vec::<String>::new();

    config.push("[Peer]".to_string());

    peer.iter()
        .filter(|(k, _)| {
            vec![
                "AllowedIPs",
                "Endpoint",
                "PublicKey",
                "PresharedKey",
                "PersistentKeepalive",
            ]
            .contains(&&k[..])
                && !k.starts_with("_")
        })
        .for_each(|(k, v)| {
            config.push(format!(
                "{} = {}",
                k,
                match &v {
                    v if v.is_str() => format!("{}", v.as_str().unwrap().to_string()),
                    v if v.is_integer() => v.as_integer().unwrap().to_string(),
                    v if v.is_bool() => v.as_bool().unwrap().to_string(),
                    _ => panic!("Unsupported type"),
                }
            ));
        });

    if let Some(Value::Boolean(is_route)) = interface.get("_route") {
        if *is_route {
            config.push(format!("AllowedIPs = {}", peer["Address"].as_str().unwrap().split("/").take(1).collect::<Vec<&str>>().get(0).unwrap()));
        }
    }

    config
}
