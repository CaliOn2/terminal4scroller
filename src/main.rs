use std::{
    env,
    io::{
        stdout,
        Write,
    },
    process::Command,
    thread,
    time,
};

struct Displayfield {
    displayring: Vec<Vec<char>>,
    displaybuffer: Vec<char>,
    ringpointer: usize,
    xsize: usize,
    ysize: usize,
}

impl Displayfield {

    fn ring2buff(&mut self) {
        let mut startindex: usize = 0;
        let mut line: &Vec<char>;
        let mut linenr: usize = 0;
        let mut offset: usize;
        while linenr < self.ysize {
            line = &self.displayring[linenr];
            offset = self.xsize - self.ringpointer;
            self.displaybuffer[startindex..startindex + offset].copy_from_slice(&line[self.ringpointer..self.xsize]);
            startindex += offset;
            offset = self.xsize - offset;
            self.displaybuffer[startindex..startindex + offset].copy_from_slice(&line[0..self.ringpointer]);
            startindex += offset;
            linenr += 1;
        }
    } 

    fn write(&mut self, linenr: usize, charval: char) {
        self.displayring[linenr][self.ringpointer] = charval;
    } 

    fn advanceringpointer(&mut self) {
        self.ringpointer = (self.ringpointer + 1) % self.xsize;
    }

    fn init(&mut self, x: usize, y: usize) {
        self.xsize = x;
        self.ysize = y;
        for yindex in 0..self.ysize {
            if self.displayring.len() < self.ysize {
                self.displayring.push(Vec::<char>::new());
            }
            for _ in 0..self.xsize {
                if self.displayring[yindex].len() < self.xsize {
                    self.displayring[yindex].push(' ');
                    self.displaybuffer.push(' ');
                }
            }
        }
    }
}
 
fn curlFilterComment (board: &str, page: u8) -> Vec<String> {
    const GREATERTHAN: [u8; 3] = ['g' as u8, 't' as u8, ';' as u8,];
    const APOSTROPHE: [u8; 5] = ['#' as u8, '0' as u8, '3' as u8, '9' as u8, ';' as u8,];
    const QUOTATION: [u8; 5] = ['q' as u8, 'u' as u8, 'o' as u8, 't' as u8, ';' as u8,];
    let data: Vec<u8> = f4chanrequester(board, page);
    let mut comments: Vec<String> = vec![String::new()];
    let mut commentindex: usize = 0;
    let mut dataindex: usize = 0;

    while dataindex < (data.len() - 30) {
        if commentstarted(&data[dataindex..dataindex + 5]) { 
            if comments.len() <= commentindex {
                comments.push(String::new());
            }
            dataindex += 7;
            while !(data[dataindex] == '"' as u8) {
                if data[dataindex] == ('<' as u8) {
                    while data[dataindex] != ('>' as u8) {
                        dataindex += 1;
                    }
                    //comments[commentindex].push_str("  ");
                } else if data[dataindex] == '&' as u8 {
                    dataindex += 1;
                    if textcheck(&data[dataindex..dataindex + 3], &GREATERTHAN) {
                        comments[commentindex].push_str(" > ");
                        dataindex += 2;
                    } else if textcheck(&data[dataindex..dataindex + 5], &APOSTROPHE) {
                        comments[commentindex].push('\'');
                        dataindex += 4;
                    } else if textcheck(&data[dataindex..dataindex + 5], &QUOTATION) {
                        comments[commentindex].push('"');
                        dataindex += 4;
                    }
                } else if data[dataindex] != '\\' as u8 {
                    comments[commentindex].push(data[dataindex] as char);
                }
                dataindex += 1;
            }
            comments[commentindex].push_str("                   ");
            dataindex += 20;
            commentindex += 1;
        }
        dataindex += 1;
    }
    return comments;
}


fn main() {
    let board: String = open();

    let mut _page: u8 = 1;
    let mut _activestack: usize = 1;
    let mut commentVault: [Vec<String>; 2] = [
        curlFilterComment(&board, _page),
        Default::default()
    ];
    let mut _activecomments: Vec<String> = vec![String::new(); 59];

    let mut display: Displayfield = Displayfield {
        displayring: Default::default(),
        displaybuffer: Default::default(),
        ringpointer: 0,
        xsize: 0,
        ysize: 0,
    };

    display.init(238, 59);
    
    loop {
        display.advanceringpointer();
        for index in 0..display.ysize {
            if _activecomments[index].is_empty() {
                if commentVault[_activestack].is_empty() {
                    // ----------This Needs To Be A Seperate Thread-----------
                    _page = (_page % 3) + 1;
                    commentVault[_activestack] = curlFilterComment(&board, _page);
                    // -------------------------------------------------------
                    //activestack = activestack == 0 ? 1 : 0;
                }
                _activecomments[index] = commentVault[_activestack].pop().unwrap();
            }
            display.write(index, _activecomments[index].chars().nth(0).unwrap());
            _activecomments[index].remove(0);
        }
        display.ring2buff();
        print!("{}", display.displaybuffer.iter().collect::<String>());
        std::thread::sleep(time::Duration::from_millis(100));
    }
}


fn f4chanrequester (board: &str, page: u8) -> Vec<u8> { 
    let link: String = format!("https://a.4cdn.org/{}/{}.json", board, page.to_string());

    let output = Command::new("curl")
        .arg(link)
        .output()
        .expect("failed to execute process");

    return output.stdout;
}

fn textcheck (data: &[u8], entity: &[u8]) -> bool { 
    for i in 0..entity.len() - 1 {
        if data[i] != entity[i] {
            return false;
        }
    }
    return true;
}

fn commentstarted (data: &[u8]) -> bool { 
    const COMVALUES: [u8; 6] = ['"' as u8, 'c' as u8, 'o' as u8, 'm' as u8, '"' as u8, ':' as u8];

    return textcheck(data, &COMVALUES);
}

fn commentclosed (data: &[u8]) -> bool {
    const FILEVALUES: [u8; 11] = ['"' as u8, 'f' as u8, 'i' as u8, 'l' as u8, 'e' as u8, 'n' as u8, 'a' as u8, 'm' as u8, 'e' as u8, '"' as u8, ':' as u8];
    const TIMEVALUES: [u8; 7] = ['"' as u8, 't' as u8, 'i' as u8, 'm' as u8, 'e' as u8, '"' as u8, ':' as u8];
    
    if textcheck(data, &FILEVALUES) {
        return true;
    } else {
        return textcheck(data, &TIMEVALUES);
    }
}

fn open() -> String { 
    const POSSIBLE_COLORS: [&str; 8] = ["0", "1", "2", "3", "4", "5", "6", "7"];
    const POSSIBLE_BOARDS: [&str; 77] = ["a", "b", "c", "d", "e", "f", "g", "gif", "h", "hr", "k", "m", "o", "p", "r", "s", "t", "u", "v", "vg", "vm", "vmg", "vr", "vrpg", "vst", "w", "wg", "i", "ic", "r9k", "s4s", "vip", "qa", "cm", "hm", "lgbt", "y", "3", "aco", "adv", "an", "bant", "biz", "cgl", "ck", "co", "diy", "fa", "fit", "gd", "hc", "his", "int", "jp", "lit", "mlp", "mu", "n", "news", "out", "po", "pol", "pw", "qst", "sci", "soc", "sp", "tg", "toy", "trv", "tv", "vp", "vt", "wsg", "wsr", "x", "xs"];
    let mut args = env::args().skip(1);
    let mut color_number: String = "3".to_string();
    let mut board: String = "r9k".to_string();

    while let Some(arg) = args.next() {
        match &arg[..] {
            "-h" | "--help" => {
                println!("
This is a Program made as a terminal 4Chan screensaver
Possible arguments are:
-c --color with ansi colors as options
-b --board with 4chan board's as options (b, vm, r9k, etc...)");
            },
            "-c" | "--color" => {
                color_number = args.next().expect("Expected Color!").to_string();
                if let None = POSSIBLE_COLORS.iter().find(|&&item| item == color_number) {
                    panic!("Bad Color!");
                }
            },
            "-b" | "--board" => {
                board = args.next().expect("Expected Board!").to_string();
                if let None = POSSIBLE_BOARDS.iter().find(|&&item| item == board) {
                    panic!("Bad Board!");
                }
            },
            _ => {
                if arg.starts_with('-') {
                    println!("Unkown argument {}", arg);
                } else {
                    println!("Unkown positional argument {}", arg);
                }
            },
        }
    }

    //ansi escape codes
    // ESC = "\x1B"
    // CSI = ESC + "["
    // terminal_color = CSI + color_number + "m"
    // CURSOR_HIDE = CSI + "?25l"
    // CURSOR_HOME = CSI + "1;1H"
    // SCREEN_CLEAR = CSI + "2J"
    // SCREEN_BUF_ON = CSI + "?1049h"
    // SCREEN_BUF_ON, CURSOR_HIDE, CURSOR_HOME, SCREEN_CLEAR, terminal_color
    println!("\x1B[?1049h\x1B[?251\x1B[1;1H\x1B[2J\x1B[3{}m", color_number);

    ctrlc::set_handler(move || {
        close();
    })
    .expect("Error setting Ctrl-C handler");

    return board;
}

fn close() {
    println!("\x1B[?1049l\x1B[?25h");
    std::process::exit(0x0100);
}
