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
    -r, --removerepeats  set to not print strings that are repeated more than 10 times in a row. Note that this has a SMALL (1/590 trillion) chance of causing strings not to print due to hash collisions. 
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
    flag_removerepeats: bool,
}

fn main(){
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let filename = args.arg_file.clone();

    if args.flag_help {
        //do something
    }

    if args.flag_version {
        const VERSION: &'static str = env!("CARGO_PKG_VERSION");
        println!("RStrings {}", VERSION);
        std::process::exit(0);        
    }

    if filename.len() == 0 { //if no filename specified, then we assume there should be something in stdin
        let mut bytes: Vec<u8> = Vec::new(); //blank vector of u8s
        let mut reader = io::stdin();
        bytes = match reader.read_to_end(&mut bytes) { //read the whole file
            Ok(x) => bytes, //standard ok() err()
            Err(_) => panic!("Failed to read the file!"), //panic if we can't read from the file
        };
        println!("Successfully read input from stdin, starting to search. ");
        searchFile(bytes, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats);
        std::process::exit(0);
    }
    

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats);
}

fn fastBadHash(str: String) -> u8 { //horrible god awful hashing algorithm but it is fast and good enough (good distribution in the 1 through 30 range so the probability of 10 matches is 1/590 trillion)
    let bytes = str.into_bytes();
    let mut hash: u8 = 0;
    let mut lastByte: u8 = 0;
    for byte in bytes {
        hash = byte ^ lastByte; 
        lastByte = byte;
    } 
    return hash;     
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
            if size > (numBytes - 1) as usize {   //if it is long enough, then check if we should check if it is null terminated; -1 is to fix OBO error so it will print things with 4 printable characters and 1 nullbyte
                if !nullBytes { //if nullBytes == true, then don't check for them
                    if file[index+i] == 0 { //null terminated
                        isFound = true; 
                    }
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

fn searchFile(file: Vec<u8>, numBytes: i32, nullBytes: bool, printFile: bool, filename: String, printLocation: bool, removeRepeats: bool) { //given a vector of u8 will search the file
    let mut hashList: Vec<u8> = Vec::new(); //serves as a cache of the last 10 hashes so we can avoid repeats
    hashList.push(256u8); //256 is an impossible? value from our hashing algorithm so we start it with that as a starting point
    let mut haveFoundAString = false; //used so we can suggest the --nullbytes flag when it is needed 
    let mut numToSkip = 0;  //the number to skip (used when we find a 5 character string so we don't then print a 4 character string followed by a 3 character and so on
    for (index,char) in file.iter().enumerate() { //index,char b/c we need both
        if numToSkip > 0 { //if we need to skip, do so
            numToSkip -= 1; //decrement it so we don't skip forever 
        }
        else { //if not skipping: 
            let temp = checkForString(file.clone(), index, numBytes, nullBytes); //temp is a tuple; temp.0 is whether or not we found one; temp.1 is the length of the string we found 
            if temp.0 { //if temp.0 is true then we found a string
                haveFoundAString = true;
                let foundString: String = getString(file.clone(), index as u64, index as u64+temp.1);
                let hash: u8 = fastBadHash(foundString.clone()); //get the hash of the string (via a *horrible* but fast hashing algorithm)
                let mut allHashesEqual = true; 
                for tempHash in hashList.clone() {
                    if tempHash != hash {
                        allHashesEqual = false; //if any of the hashes don't match, then we don't skip
                    }
                }
                if ! (allHashesEqual && removeRepeats /*We found something that is being duplicated*/) { //if we don't need to skip it
                    if printFile && !printLocation {
                        println!("{1}:{0}", foundString, filename);
                    }
                    else { //if else b/c of the borrower 
                        if !printFile && printLocation {
                            println!("{1}:{0}", foundString, index);
                        }
                        else {
                            if printFile && printLocation {
                                println!("{1}:{2}:{0}", foundString, filename, index);
                            }
                            else {
                                if !printFile && !printLocation {
                                    println!("{}", foundString);
                                }
                            }
                        }
                    }
                }
                if hashList.len() > 10 { //only if there are 10 cached hashes should we start removing them
                    hashList.remove(0); //remnove the first (oldest) element in the cache
                }
                hashList.push(hash); //add the latest hash to the end of the cache
                numToSkip = temp.1; //now we need to skip the length of the string
            }
        }
    }
    if !haveFoundAString {
        println!("Failed to find any strings. Are the strings null terminated? Try the --nullbytes flag to disable the null byte requirement. ")
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
