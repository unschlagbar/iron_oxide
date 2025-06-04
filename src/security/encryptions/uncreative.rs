
pub struct Uncreative;

impl Uncreative {
    const HEADER: [u8; 8]  = [54, 12, 74, 124, 74, 91, 0, 80];

    pub fn encrypt_data(data: &[u8], len: usize, key: u16) -> Option<Vec<u8>> {

        println!("key: {}", key);
    
        if len < 1 {
            return None;
        }
    
        let mut encrypted_content: Vec<u8> = Vec::with_capacity(len as usize + 10);
    
        encrypted_content.extend_from_slice(&Self::HEADER);
        encrypted_content.extend_from_slice(&(key ^ 0b01011101100010110).to_le_bytes());

        let mut i = 0;
    
        for char in data {
    
            //Diese Prossez ist nur dank Phillips aufmerksamen handeln und denken und seiner auffassungs gabe möglich!
            let mut encrypted_char: u8 = *char << 4 | *char >> 4;
            encrypted_char ^= (key << (i * 8)) as u8;
      
            encrypted_content.push(encrypted_char);
            i = (i + 1) % 2
        }
    
        Some(encrypted_content)
    }
    
    pub fn decrypt_data(data: &[u8], len: usize) -> Option<Vec<u8>> {
    
        if len > 10 {
            if data[0..8] != Self::HEADER {
                return None;
            }
        } else {
            return None;
        }
    
        let key: u16 = (data[8] as u16 | (data[9] as u16) << 8 ) ^ 0b01011101100010110;
        let mut decrypted_content: Vec<u8> = Vec::with_capacity(len - 10);

        println!("key: {}", key);
    
        for i in 10..len {
            
            let mut char = data[i];
            char ^= (key << (i % 2 * 8)) as u8;

            char = char << 4 | char >> 4;
    
            decrypted_content.push(char);
        }
    
        Some(decrypted_content)
    }
}