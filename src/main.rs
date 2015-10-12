extern crate docopt;
use docopt::Docopt;

extern crate rustc_serialize;

use std::io; //for stdin
use std::env; 
use std::path::Path; //for filenames 
use std::fs::File; //for the file
use std::io::BufReader; //buffered reader so we can handle large files
use std::io::Read; //to read from the above file

const USAGE: &'static str = "
Usage: rustStrings [options] [<file>]

Options:
    -b, --bytes=<num>  set the number of printable bytes needed for something to qualify as a string [default: 4]
    -n, --nullbytes  set to disable the null byte requirement
    -f, --filename  print the name of the file before each line
    -l, --location  print the location of the string in the binary (bytes past starting point)
    -h, --help  display this help and exit
    -v, --version  output version information and exit
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_file: String,
    flag_bytes: i32,
    flag_nullbytes: bool,
    flag_filename: bool,
    flag_location: bool,
    flag_help: bool,
    flag_version: bool,
}

fn main(){
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let filename = args.arg_file.clone();

    if args.flag_version {
        const VERSION: &'static str = env!("CARGO_PKG_VERSION");
        println!("RStrings {}", VERSION);
        std::process::exit(0);        
    }

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location);
}

fn isPrintable(char: u8) -> bool { //checks whether the character (as a u8) is printable
    if char >= 32u8 && char <= 126u8 {  //printable is 32..126
        return true;
    }
    return false;
}

fn checkForString(file: Vec<u8>, index: usize, numBytes: i32, nullBytes: bool) -> (bool, u64) { //bool is whether or not we did, u64 is the length of it if we did
    let mut isFound = false; //by default we never found it
    let mut size = 0;   //size=0 is the default
    let mut i = 0;  //used in the loop{} structure as a counter
    loop {
        if isPrintable(file[index+i]){  //if it is printable, then just loop to go to the next one
            i += 1; //must increment it so we go to the next character in the file
        }
        if !isPrintable(file[index+i]){ //if it isn't printable then check if it is long enough yet
            size = i;
            if size > 5 as usize {   //if it is long enough, then check if we should check if it is null terminated
                if file[index+i] == 0 { //null terminated
                    isFound = true; 
                }
                else {
                    isFound = true; 
                }
            } 
            break; //once we find a non-printable character we break
        }
    }
    return (isFound, size as u64)   //return it as a u64 so it is sufficiently large
}

fn searchFile(file: Vec<u8>, numBytes: i32, nullBytes: bool, printFile: bool, filename: String, printLocation: bool) { //given a vector of u8 will search the file
    let mut numToSkip = 0;  //the number to skip (used when we find a 5 character string so we don't then print a 4 character string followed by a 3 character and so on
    for (index,char) in file.iter().enumerate() { //index,char b/c we need both
        if numToSkip > 0 { //if we need to skip, do so
            numToSkip -= 1; //decrement it so we don't skip forever 
        }
        else { //if not skipping: 
            let temp = checkForString(file.clone(), index, numBytes, nullBytes); //temp is a tuple; temp.0 is whether or not we found one; temp.1 is the length of the string we found 
            if temp.0 { //if temp.0 is true then we found a string
                println!("{}", getString(file.clone(), index as u64, index as u64+temp.1));
                numToSkip = temp.1; //now we need to skip the length of the string
            }
        }
    }
}

fn getString(file: Vec<u8>, startIndex: u64, endIndex: u64) -> String { //given the indexes in the file and the file, return the string
    let mut str = String::new(); //blank string
    for i in startIndex..endIndex { //go through each character that should be part of the string
        str.push(file[i as usize] as char); //and add them via String.push(char)
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
