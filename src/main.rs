use std::io; 
use std::env; 
use std::path::Path; 
use std::fs::File; 
use std::io::BufReader; 
use std::io::Read; 

fn main(){
    let mut args: Vec<_> = env::args().collect(); 
    args.remove(0); 
    if args.len() != 1 { 
        panic!("This program accepts 1 argument, you supplied it with {} arguments", args.len());
    }
    let filename = args[0].clone(); 

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file);
}

fn checkForString(file: Vec<u8>, index: usize) -> (bool, u64) { 

}

fn searchFile(file: Vec<u8>) { 

}

fn openFile(filename: String) -> Vec<u8> { 

}
