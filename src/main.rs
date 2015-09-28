use std::io; 
use std::env; 
use std::path::Path; 
use std::fs::File; 
use std::io::BufReader; 
use std::io::Read; 

fn main(){
    let filename = String::from("a.out"); 

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file);
}

fn isPrintable(char: u8) -> bool { 
    if char >= 32u8 && char <= 126u8 {
        return true;
    }
    return false;
}

fn checkForString(file: Vec<u8>, index: usize) -> (bool, u64) { 
    let mut isFound = false;
    let mut size = 0;   
    let mut i = 0;  
    loop {
        if isPrintable(file[index+i]){  
            i += 1; 
        }
        if !isPrintable(file[index+i]){ 
            size = i;
            if size > 5 as usize {   
                if file[index+i] == 0 { 
                    isFound = true; 
                }
                else {
                    isFound = true; 
                }
            } 
            break; 
        }
    }
    return (isFound, size as u64)   
}

fn searchFile(file: Vec<u8>) { 
    for (index,char) in file.iter().enumerate() { 
        let temp = checkForString(file.clone(), index); 
        if temp.0 { 
            println!("{}", getString(file.clone(), index as u64, index as u64+temp.1)); 
        }
    }
}

fn getString(file: Vec<u8>, startIndex: u64, endIndex: u64) -> String { 
    let mut str = String::new(); 
    for i in startIndex..endIndex { 
        str.push(file[i as usize] as char); 
    }
    return str;
}

fn openFile(filename: String) -> Vec<u8> { //returns a vector of bytes (where byte == u8) in the file with the given filename
    let mut file = match File::open(filename) { //this is creating the file variable
        Ok(file) => file,                       //standard ok() and Err() syntax to check for errors
        Err(_) => panic!("Failed to open the file!"), //if we can't open it, then panic
    };

    let mut bytes: Vec<u8> = Vec::new(); //blank vector of u8s
    let mut reader = BufReader::new(file); //buffered reader so we can handle large files
    return match reader.read_to_end(&mut bytes) { //read the whole file
        Ok(x) => bytes, //standard ok() err()
        Err(_) => panic!("Failed to read the file!"), //panic if we can't read from the file
    };
    return bytes;
}
