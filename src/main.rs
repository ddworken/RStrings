#![allow(non_snake_case)]

extern crate docopt;
use docopt::Docopt;

extern crate rustc_serialize;

use std::io; //for stdin
use std::fs::File; //for the file
use std::io::BufReader; //buffered reader so we can handle large files
use std::io::Read; //to read from the above file
use std::str; //to read utf-8

use std::thread; //for concurrency 
extern crate num_cpus; //for autodetection of cpu count 

const USAGE: &'static str = "
Usage: rustStrings [options] [<file>]

Options:
    -b, --bytes=<num>  set the number of printable bytes needed for something to qualify as a string [default: 4]
    -t, --threads=<num>  set the number of threads to use. Use 0 to automatically detect the optimal number of threads. Note if threads > 1 than the order of the strings found may not match the order of the strings in the file. [default: 1]
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
    flag_threads: i32,
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
        searchFile(bytes, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats, args.flag_utf8, args.flag_threads, false);
        std::process::exit(0);
    }
    

    println!("Opening {} to search it for strings...", filename);   
    let file = openFile(filename.clone());
    println!("Opened {}. ", filename);
    searchFile(file, args.flag_bytes, args.flag_nullbytes, args.flag_filename, filename.clone(), args.flag_location, args.flag_removerepeats, args.flag_utf8, args.flag_threads, false);
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
    let lenFile = file.len();   //used for out of range prevention
    let buf = &[file[index], file[index+1], file[index+2], file[index+3]];
    for i in 1..4 { //1,2,3
        let s = match str::from_utf8(&buf[0..i]) { //convert it to a UTF-8
            Ok(n) => {},
            Err(err) => { //keep track of whether or not we found a UTF-8 char
                if foundUTF8 == false {
                    foundUTF8 = true;
                    len = i+1;
                }
            },
        };
    }
    return (foundUTF8, len);
}

fn checkForString(file: Vec<u8>, index: usize, numBytes: i32, nullBytes: bool, utf8: bool) -> (bool, u64) { //bool is whether or not we did, u64 is the length of it if we did
    let mut isFound = false; //by default we never found it
    let mut size = 0;   //size=0 is the default
    let mut i = 0;  //used in the loop{} structure as a counter
    if !utf8 {
        loop {
            if index+i >= file.len() {
                break;
            }
            if isPrintableASCII(file[index+i]){  //if it is printable, then just loop to go to the next one
                i += 1; //must increment it so we go to the next character in the file
            }
            else {
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

fn searchFile(file: Vec<u8>, numBytes: i32, nullBytes: bool, printFile: bool, filename: String, printLocation: bool, removeRepeats: bool, utf8: bool, mut threads: i32, inThread: bool) { //given a vector of u8 will search the file
    if threads == 1 {
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
        if !haveFoundAString && !inThread {
            println!("Failed to find any strings. Are the strings null terminated? Try the --nullbytes flag to disable the null byte requirement. If you need UTF-8 support, use the --utf8 flag to enable utf8 support. ")
        }
    }
    if threads == 0 {
        threads = num_cpus::get() as i32 * 16;
    }
    if threads > 1 {
        let mut files: Vec<Vec<u8>> = Vec::new(); //a vector of files (which can each be treated as a normal file)
        let lenOriginaFile = file.len(); 
        if lenOriginaFile < 1000 { //if it is less than 1KB
            panic!("Cannot use multiple threads on files less than 1KB in size. ");
        }
        let lenChunkPerThread = lenOriginaFile / threads as usize; //the number of bytes given to each thread
        for i in 0..threads { //initialize the vectors that will each hold the bytes in 1 file
            files.push(Vec::new());
        }
        let mut currentThread = 0;
        for (index,byte) in file.iter().enumerate() { //splits the file into n files 
            if index > 0 && index % lenChunkPerThread == 0 {
                if !(currentThread == threads-1) {  //makes it so the extra n bytes at the end (from integer division) are just stuck on the last thread
                    currentThread += 1
                }
            }
            files[currentThread as usize].push(*byte);
        }
        for vec in files.iter() {
            println!("Length: {:?}", vec.len());
        }
        let mut children = vec![];
        let isInThread = true; 
        for i in 0..threads {
            let tempFile = files[i as usize].clone();
            let tempFilename = filename.clone();
            children.push(thread::spawn(move || {
                searchFile(tempFile, numBytes, nullBytes, printFile, tempFilename, printLocation, removeRepeats, utf8, 1, isInThread);
            }));
        }
        for child in children {
            let _ = child.join();
        }
    }
}

fn getString(file: Vec<u8>, startIndex: u64, endIndex: u64) -> String { //given the indexes in the file and the file, return the string
    let mut vec: Vec<u8> = Vec::new();
    for i in startIndex..endIndex { //go through each character that should be part of the string
        vec.push(file[i as usize]);
    }
    let byteArr = &vec[..];
    let str = match str::from_utf8(byteArr) {
                    Ok(n) => n,
                    Err(err) => "",
                };
    return String::from(str);
}

fn openFile(filename: String) -> Vec<u8> { //returns a vector of bytes (where byte == u8) in the file with the given filename
    let file = match File::open(filename) { //this is creating the file variable
        Ok(file) => file,                       //standard ok() and Err() syntax to check for errors
        Err(_) => panic!("Failed to open the file!"), //if we can't open it, then panic
    };

    let mut bytes: Vec<u8> = Vec::new(); //blank vector of u8s
    let mut reader = BufReader::new(file); //buffered reader so we can handle large files
    return match reader.read_to_end(&mut bytes) { //read the whole file
        Ok(x) => bytes, //standard ok() err()
        Err(_) => panic!("Failed to read the file!"), //panic if we can't read from the file
    };
}

#[cfg(test)]
mod tests {
    use std::process::Command; //for executing external python test suite

    use std::str; //to read utf-8

    use super::getString;
    use super::isUTF8;
    use super::checkForString; 
    use super::fastBadHash;
    use super::openFile; 
    use super::isPrintableASCII; 

    #[test]
    fn testHelloWorld() {
        let status = Command::new("cargo").arg("run").arg("--").arg("./testCases/a.out").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(36246, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testHelloWorldUnicode() {
        let status = Command::new("cargo").arg("run").arg("--").arg("--utf8").arg("./testCases/unicodeBinary").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(14675, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testNullBytes() {
        let status = Command::new("cargo").arg("run").arg("--").arg("--nullbytes").arg("--utf8").arg("./testCases/short").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(61590, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testRemoveRepeats() {
        let status = Command::new("cargo").arg("run").arg("--").arg("--removerepeats").arg("--nullbytes").arg("./testCases/repeated").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(17811, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testLocationAndFilename() {
        let status = Command::new("cargo").arg("run").arg("--").arg("--location").arg("--filename").arg("--nullbytes").arg("./testCases/repeated").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(29860, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testThreads() {
        let status = Command::new("cargo").arg("run").arg("--").arg("--threads=4").arg("./testCases/a.out").output().unwrap_or_else(|e| {
            panic!("failed to execute process: {}", e)
        });
        let str = match str::from_utf8(&status.stdout) {
                    Ok(n) => n,
                    Err(err) => "",
                };
        let output = String::from(str);
        assert_eq!(50093, fastBadHash(output)); //easier to embed a hash of the output than the output, the output is stored in the testcases directory. If this test fails check the cached output
    }

    #[test]
    fn testGetString() {
        let vec = vec![104u8, 105u8];
        assert_eq!(String::from("hi"), getString(vec, 0, 2));
    }

    #[test]
    fn testIsUTF8() {
        let vec = vec![62u8, 194u8, 162u8, 62u8];
        assert_eq!((true, 3), isUTF8(vec.clone(), 0));
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

    #[test]
    fn testOpenFile() {
        openFile(String::from("/home/david/code/strings/RStrings/src/main.rs"));
    }

    #[test]
    fn testIsASCII() {
        assert_eq!(true, isPrintableASCII(97u8));
        assert_eq!(false, isPrintableASCII(10u8));
    }
}
