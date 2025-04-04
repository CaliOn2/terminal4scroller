use std::{
    env,
    io::{
        stdout,
        Write,
    },
    process::Command,
    thread,
    time,
    alloc::{
        alloc,
        Layout,
        dealloc,
    },
};

struct Displayfield { 
    activecomments: Vec<Vec<(*mut Comment, Layout)>>,
    displaybuffer: Vec<char>,
    buffindex: usize,
    xsize: usize,
    ysize: usize,
}

struct Comment {
    content: Vec<char>,
    width: usize, // this exists because parsing utf-8 is a bitch and multi byte chars are too
    color: char,
    reverseindex: isize,
}

fn min(num1: isize, num2: isize) -> isize {
    if num1 < num2 {
        return num1;
    }
    return num2;
}

fn max(num1: isize, num2: isize) -> isize {
    if num1 > num2 {
        return num1;
    }
    return num2;
}

impl Displayfield {

    unsafe fn comment2buff(&mut self, comment: (*mut Comment, Layout), textlength: usize) -> usize {
        let x = self.xsize;
        let commentstopoffset: usize = <isize as TryInto<usize>>::try_into(min((*comment.0).reverseindex, 0) + <usize as TryInto<isize>>::try_into((*comment.0).width).unwrap()).unwrap(); // comment.reverseindex is negative when its out of bounds 
        let commentstartoffset: usize = (max((*comment.0).reverseindex + <usize as TryInto<isize>>::try_into((*comment.0).width - x).unwrap(), 0)).try_into().unwrap(); // Todo: explain this better
        let commentfillength: usize = commentstopoffset - commentstartoffset;

        self.buffindex += self.changecolor((*comment.0).color, self.buffindex);

        self.displaybuffer[self.buffindex..(self.buffindex + commentfillength)].copy_from_slice(&(*comment.0).content[commentstartoffset..commentstopoffset]);

        self.buffindex += commentfillength;

        (*comment.0).reverseindex += 1;

        return textlength + commentfillength;
    }

    //fn edgecomment2buff(&mut self, ) -> usize {
        
    //}

    fn changecolor(&mut self, color: char, index: usize) -> usize {
        const ANSICOLOR: [char; 12] = ['\\','[','\\', '0', '3', '3', '[', '0', '0', 'm', '\\', ']'];
        self.displaybuffer[index..index + 12].copy_from_slice(&ANSICOLOR);
        self.displaybuffer[index + 8] = color;
        return 12;
    }

    fn addcomment(&mut self, lineindex: usize, comment: (*mut Comment, Layout)) {
        // adds item to the start of the list and moves ever other item by 1 so the list size
        // increases by 1
        let line = &mut (self.activecomments[lineindex]);
        let mut commentsindex: usize = line.len() - 1;

        line.push(line[commentsindex]);
        
        while commentsindex > 1 {
            commentsindex -= 1;
            line[commentsindex + 1] = line[commentsindex];
        }

        line[0] = comment;
    }

    fn init(&mut self, x: usize, y: usize) {
        self.xsize = x;
        self.ysize = y;
        let mut commentmem: Layout;
        let mut ptrcomment: *mut Comment;
        let mut newline: Vec<(*mut Comment, Layout)>;

        while self.activecomments.len() < y {
            newline = vec![];
            commentmem = Layout::new::<Comment>();
            unsafe {
                ptrcomment = alloc(commentmem) as *mut Comment;
                (*ptrcomment).content = Vec::new();
                (*ptrcomment).content.push(' ');
                (*ptrcomment).width = 1;
                (*ptrcomment).color = '0';
                (*ptrcomment).reverseindex = -1;
                newline.push((ptrcomment, commentmem));
                self.activecomments.push(newline);
            }
            print!("othertest");
        }
        while self.activecomments.len() > y {
            let mut emptythislist: Vec<(*mut Comment, Layout)> = self.activecomments.pop().unwrap();
            while emptythislist.len() > 0 {
                let commentmem: (*mut Comment, Layout) = emptythislist.pop().unwrap();
                unsafe {
                    dealloc(commentmem.0 as *mut u8, commentmem.1);
                }
            }
        }

        while self.displaybuffer.len() < (self.ysize * 2 * self.xsize) {
            self.displaybuffer.push(' ');
        }
    }
}
 
fn curl_filter_comment (board: &str, page: u8) -> Vec<(*mut Comment, Layout)> {
    const GREATERTHAN: [u8; 3] = ['g' as u8, 't' as u8, ';' as u8,];
    const APOSTROPHE: [u8; 5] = ['#' as u8, '0' as u8, '3' as u8, '9' as u8, ';' as u8,];
    const QUOTATION: [u8; 5] = ['q' as u8, 'u' as u8, 'o' as u8, 't' as u8, ';' as u8,];
    let data: Vec<u8> = f4chanrequester(board, page);
    let mut comments: Vec<(*mut Comment, Layout)> = vec![];
    let mut colorcounter: u8 = 0;
    let mut dataindex: usize = 0;
    let mut commentwidth: usize = 0;
    let mut ptrcomment: *mut Comment;
    let mut commentmem: Layout;

    while dataindex < (data.len() - 30) {
        if commentstarted(&data[dataindex..dataindex + 5]) { 
            
            print!("Test");
            commentmem = Layout::new::<Comment>();
            unsafe {
                ptrcomment = alloc(commentmem) as *mut Comment;
                (*ptrcomment).color = (49 + colorcounter) as char;
                (*ptrcomment).content = Vec::new();
            

                colorcounter = (colorcounter + 1) % 10;

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

                            (*ptrcomment).content.push(' ');
                            (*ptrcomment).content.push('>');
                            (*ptrcomment).content.push(' ');


                            commentwidth += 2;
                            dataindex += 2;
                        } else if textcheck(&data[dataindex..dataindex + 5], &APOSTROPHE) {

                            (*ptrcomment).content.push('\'');

                            dataindex += 4;
                        } else if textcheck(&data[dataindex..dataindex + 5], &QUOTATION) {

                            (*ptrcomment).content.push('"');

                            dataindex += 4;
                        }
                    } else if data[dataindex] != '\\' as u8 {

                        (*ptrcomment).content.push(data[dataindex] as char);

                    }

                    if (data[dataindex] as u8 >> 6) != 0b10 {
                        commentwidth += 1;
                    }

                    dataindex += 1;
                }
                dataindex += 20;

                (*ptrcomment).reverseindex = -(<usize as TryInto<isize>>::try_into(commentwidth)).unwrap();
                (*ptrcomment).width = commentwidth;
            
                comments.push((ptrcomment, commentmem));
            }
        }
        dataindex += 1;
    }
    return comments;
}


fn main() {
    let board: String = open();

    let mut _page: u8 = 1;
    let mut _activestack: usize = 1;
    let mut comment_vault: [Vec<(*mut Comment, Layout)>; 2] = [
        curl_filter_comment(&board, _page),
        Default::default()
    ];

    let mut textlength: usize; // this is a necessarry value because ansi color changes require unseen bytes

    let mut display: Displayfield = Displayfield {
        activecomments: Default::default(),
        displaybuffer: Default::default(),
        buffindex: 0,
        xsize: 0,
        ysize: 0,
    };

    display.init(238, 59);

    let mut linelength: usize;
    let mut ptrdropcom: (*mut Comment, Layout);
    
    loop {
        for lineindex in 0..display.ysize - 1 {
            textlength = 0;

            linelength = display.activecomments[lineindex].len();
            
            unsafe {

                if (*(display.activecomments[lineindex][linelength - 1].0)).reverseindex > display.xsize.try_into().unwrap() {
                    ptrdropcom = display.activecomments[lineindex].pop().unwrap();
                    //drop((*ptrdropcom).content);

                    dealloc(ptrdropcom.0 as *mut u8, ptrdropcom.1);
                    linelength -= 1;
                }

            }

            while linelength > 0  {

                linelength -= 1;

                unsafe {
                    textlength = display.comment2buff(display.activecomments[lineindex][linelength], textlength);
                }

            }
            // check if theres a leftover byte and if there is a new comment is added
            if textlength < display.xsize {
                if comment_vault[_activestack].is_empty() {
                    _page = 1 + (_page % 8);
                    curl_filter_comment(&board, _page);
                }
                display.addcomment(lineindex, comment_vault[_activestack].pop().expect("Somehow no comment found (input to addcomment)"));

                unsafe {
                    display.buffindex += display.changecolor((*display.activecomments[lineindex][0].0).color, display.buffindex);
                    display.displaybuffer[display.buffindex] = (*display.activecomments[lineindex][0].0).content[0];
                }

                display.buffindex += 1;
            }
        }

        print!("{}", display.displaybuffer[0..display.buffindex].iter().collect::<String>());

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
    const POSSIBLE_BOARDS: [&str; 77] = ["a", "b", "c", "d", "e", "f", "g", "gif", "h", "hr", "k", "m", "o", "p", "r", "s", "t", "u", "v", "vg", "vm", "vmg", "vr", "vrpg", "vst", "w", "wg", "i", "ic", "r9k", "s4s", "vip", "qa", "cm", "hm", "lgbt", "y", "3", "aco", "adv", "an", "bant", "biz", "cgl", "ck", "co", "diy", "fa", "fit", "gd", "hc", "his", "int", "jp", "lit", "mlp", "mu", "n", "news", "out", "po", "pol", "pw", "qst", "sci", "soc", "sp", "tg", "toy", "trv", "tv", "vp", "vt", "wsg", "wsr", "x", "xs"];
    let mut args = env::args().skip(1);
    let mut board: String = "r9k".to_string();

    while let Some(arg) = args.next() {
        match &arg[..] {
            "-h" | "--help" => {
                panic!("
This is a Program made as a terminal 4Chan screensaver
Possible arguments are:
-b --board with 4chan board's as options (b, vm, r9k, etc...)");
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
    println!("\x1B[?1049h\x1B[?251\x1B[1;1H\x1B[2J");

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
