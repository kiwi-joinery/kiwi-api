use rand::Rng;
use rand::distributions::Standard;

pub fn generate_token(length: u8) -> String {
    let mut rng = rand::thread_rng();
    let v: Vec<u8> = rng.sample_iter(&Standard).take(length as usize).collect();
    base64::encode(&v)
}
