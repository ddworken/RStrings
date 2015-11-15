extern crate docopt;
use docopt::Docopt;

extern crate rustc_serialize;

use std::io; //for stdin
use std::env; 
use std::path::Path; //for filenames 
use std::fs::File; //for the file
use std::io::BufReader; //buffered reader so we can handle large files
use std::io::Read; //to read from the above file
use std::str; //to read utf-8

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
    -u, --utf8  set to enable utf-8 support
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
    flag_utf8: bool, 
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
        searchFile(bytes, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats, args.flag_utf8);
        std::process::exit(0);
    }
    

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats, args.flag_utf8);
}

fn fastBadHash(str: String) -> u32 {  //bad hashing algorithm 32 bit version of: https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function
    let bytes = str.into_bytes();
    let mut hash: u64 = 2166136261;   //first constant for the 32 bit version of the hash
    for byte in bytes {
        hash = hash ^ byte as u64;              //as u64 to convert it to u8
        hash = hash * 16777619;                 //second constant for the 32 bit version of the hash
        hash = hash % 65536;
    }
    return hash as u32; //yes this is an unsafe conversion it is equivalent to % so it is fine
}

fn isPrintableASCII(char: u8) -> bool { //checks whether the character (as a u8) is printable
    if char >= 32u8 && char <= 126u8 {  //printable is 32..126
        return true;
    }
    return false;
}

fn isUTF8(file: Vec<u8>, index: usize) -> (bool, usize) {   //checks if the group of non-printable characters at index in file is a valid unicode character
    if index >= file.len(){ //check to see if we are going to search out of bounds
        return (false, 0);
    }
    let mut foundUTF8 = false;  //used to hold the current status of whether or not we found a utf-8 character
    let mut len = 0;            //used to hold the length (in bytes) of the utf-8 character
    let mut foundASCII = false; //used to keep track of if we found an ASCII character (if we did then we didn't find a UTF-8 character)
    let lenFile = file.len();   //used for out of range prevention
    /*for i in 1..4 { //utf8 characters are between 1 and 4 bytes 
        if index + 1 > lenFile {
            if !isPrintableASCII(file[index+i]) {
                foundASCII = true;
            }
        }
    }
    if foundASCII { //if we found an ASCII character, then return false since ASCII != UTF-8
        return (false, 0);
    }
    else */{
        /*
         * Below is a rather hack-ish solution. Rust requires that arrays have compile time defined lengths. 
         * from_utf8() must be called on an array (not a vector) so we try all possible array lengths 
         * (2 bytes, 3 bytes, or 4 bytes) individually. 
        */
        { //for checking for 2 byte utf-8
            if index + 1 < lenFile { //check for out of bounds
                let mut buf = &[file[index], file[index+1]]; //create the array
                let s = match str::from_utf8(buf) { //convert it to a UTF-8
                    Ok(n) => {},
                    Err(err) => { //keep track of whether or not we found a UTF-8 char
                        if foundUTF8 == false {
                            foundUTF8 = true;
                            len = 2;
                        }
                    },
                };
            }
        }
        { //for checking for 3 byte utf-8
            if index + 2 < lenFile {
                let mut buf = &[file[index], file[index+1], file[index+2]]; 
                let s = match str::from_utf8(buf) {
                    Ok(n) => {},
                    Err(err) => {
                        if foundUTF8 == false {
                            foundUTF8 = true;
                            len = 3; 
                        }
                    },
                };
            }
        }
        { //for checking for 4 byte utf-8
            if index + 3 < lenFile {
                let mut buf = &[file[index], file[index+1], file[index+2], file[index+3]]; 
                let s = match str::from_utf8(buf) {
                    Ok(n) => {},
                    Err(err) => {
                        if foundUTF8 == false {
                            foundUTF8 = true;
                            len = 4; 
                        }
                    },
                };
            }
        }
    }
    return (foundUTF8, len);
}

fn checkForString(file: Vec<u8>, index: usize, numBytes: i32, nullBytes: bool, utf8: bool) -> (bool, u64) { //bool is whether or not we did, u64 is the length of it if we did
    let mut isFound = false; //by default we never found it
    let mut size = 0;   //size=0 is the default
    let mut i = 0;  //used in the loop{} structure as a counter
    if !utf8 {
        loop {
            if isPrintableASCII(file[index+i]){  //if it is printable, then just loop to go to the next one
                i += 1; //must increment it so we go to the next character in the file
            }
            if !isPrintableASCII(file[index+i]){ //if it isn't printable then check if it is long enough yet
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
    }
    if utf8 {   //special string searching logic for utf-8 search since it is considerably slower than searching for ASCII strings
        let mut numberOfCharacters = 0;     //must keep track of the numberOfCharacters alone since num of characters != num of bytes for UTF-8
        loop {
            if isPrintableASCII(file[index+i]){  //if it is printable, then just loop to go to the next one
                i += 1; //must increment it so we go to the next character in the file
                numberOfCharacters += 1;    //found 1 character
            }
            else {
                if !isPrintableASCII(file[index+i]){
                    let mut utf8CheckerTuple = (false, 0);
                    if index+i+1 < file.len() && file[index+i+1] > 127 {
                        utf8CheckerTuple = isUTF8(file.clone(), index+i+1); //tuple of (whetherFoundUTF8, number of bytes of UTF8)
                    }
                    else {
                        utf8CheckerTuple = (false, 0); 
                    }
                    if utf8CheckerTuple.0 { //if we found a utf8 character
                        numberOfCharacters += 1;      //we found one character but more than 1 byte
                        i += utf8CheckerTuple.1 + 1; //add the number of bytes to our indexIterator
                    }
                    else { //if not then check if we have a string
                        size = i;
                        if numberOfCharacters > (numBytes -1) { //prevents out of bounds read
                            if !nullBytes { //if nullBytes == true, then don't check for them
                                if file[index+i] == 0 { //null terminated
                                    isFound = true; 
                                }
                            }
                            else {
                                isFound = true; 
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    return (isFound, size as u64)   //return it as a u64 so it is sufficiently large
}

fn searchFile(file: Vec<u8>, numBytes: i32, nullBytes: bool, printFile: bool, filename: String, printLocation: bool, removeRepeats: bool, utf8: bool) { //given a vector of u8 will search the file
    let mut hashList: Vec<u32> = Vec::new(); //serves as a cache of the last 10 hashes so we can avoid repeats
    hashList.push(0); //256 is an impossible? value from our hashing algorithm so we start it with that as a starting point
    let mut haveFoundAString = false; //used so we can suggest the --nullbytes flag when it is needed 
    let mut numToSkip = 0;  //the number to skip (used when we find a 5 character string so we don't then print a 4 character string followed by a 3 character and so on
    for (index,char) in file.iter().enumerate() { //index,char b/c we need both
        if numToSkip > 0 { //if we need to skip, do so
            numToSkip -= 1; //decrement it so we don't skip forever 
        }
        else { //if not skipping: 
            let temp = checkForString(file.clone(), index, numBytes, nullBytes, utf8); //temp is a tuple; temp.0 is whether or not we found one; temp.1 is the length of the string we found 
            if temp.0 { //if temp.0 is true then we found a string
                haveFoundAString = true;
                let foundString: String = getString(file.clone(), index as u64, index as u64+temp.1);
                let hash: u32 = fastBadHash(foundString.clone()); //get the hash of the string (via a *horrible* but fast hashing algorithm)
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
        println!("Failed to find any strings. Are the strings null terminated? Try the --nullbytes flag to disable the null byte requirement. If you need UTF-8 support, use the --utf8 flag to enable utf8 support. ")
    }
}

fn getString(file: Vec<u8>, startIndex: u64, endIndex: u64) -> String { //given the indexes in the file and the file, return the string
    let mut vec: Vec<u8> = Vec::new();
    for i in startIndex..endIndex { //go through each character that should be part of the string
        vec.push(file[i as usize]);
    }
    let byteArr = &vec[..];
    let mut str = ""; 
    str = match str::from_utf8(byteArr) {
                    Ok(n) => n,
                    Err(err) => "",
                };
    return String::from(str);
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

#[cfg(test)]
mod tests {
    use super::getString;
    use super::isUTF8;
    use super::checkForString; 
    use super::fastBadHash;

    #[test]
    fn testGetString() {
        let vec = vec![104u8, 105u8];
        assert_eq!(String::from("hi"), getString(vec, 0, 2));
    }

    #[test]
    fn testIsUTF8() {
        let vec = vec![62u8, 194u8, 162u8, 62u8];
        assert_eq!((true, 2), isUTF8(vec.clone(), 0));
        assert_eq!(String::from("¢"), getString(vec, 1, 3));
    }

    #[test]
    fn testIsNotUTF8() {
        let vec = vec![62u8, 62u8, 62u8, 62u8];
        assert_eq!((false, 0), isUTF8(vec.clone(), 0));
    }

    //searchFile(file: Vec<u8>, numBytes: i32, nullBytes: bool, printFile: bool, filename: String, printLocation: bool, removeRepeats: bool, utf8: bool)

    #[test]
    fn testCheckForString() {
        let vec = vec![10u8, 62u8, 63u8, 64u8, 65u8, 66u8, 10u8, 63u8, 64u8, 65u8, 66u8, 67u8, 0u8, 12u8];
        let numBytes = 4;
        let mut nullBytes = false; 
        let utf8 = false; 
        assert_eq!((false, 0), checkForString(vec.clone(), 0, numBytes, nullBytes, utf8));
        assert_eq!((false, 5), checkForString(vec.clone(), 1, numBytes, nullBytes, utf8));
        assert_eq!((false, 4), checkForString(vec.clone(), 2, numBytes, nullBytes, utf8));
        assert_eq!((true, 5), checkForString(vec.clone(), 7, numBytes, nullBytes, utf8));
        nullBytes = true;
        assert_eq!((true, 5), checkForString(vec.clone(), 1, numBytes, nullBytes, utf8));
    }

    #[test]
    fn testHash() {
        assert_eq!(35793, fastBadHash(String::from("testHash")));
    }

}
