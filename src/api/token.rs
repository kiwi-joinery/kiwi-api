use rand::distributions::Standard;
use rand::Rng;

pub fn generate_token(length: u8) -> String {
    let rng = rand::thread_rng();
    let v: Vec<u8> = rng.sample_iter(&Standard).take(length as usize).collect();
    base64::encode(&v)
}
