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
    rc::Rc,
    cell::RefCell,
};

struct Displayfield { 
    activecomments: Vec<Vec<Rc<Comment>>>,
    displaybuffer: Vec<u8>,
    buffindex: usize,
    xsize: usize,
    ysize: usize,
}

struct Comment {
    content: Vec<u8>,
    width: usize, // this exists because parsing utf-8 is a bitch and multi byte chars are too
    color: u8,
    rearoffset: RefCell<usize>,
    reverseindexraw: RefCell<isize>, 
    frontoffset: RefCell<usize>,
    frontoffsetraw: RefCell<usize>
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

    fn comment2buff(&mut self, comment: Rc<Comment>) -> usize {
        if comment.width == 0 {
            return 0;
        }
        let x = self.xsize;
        let mut offset: isize;
 
        self.changecolor(comment.color, self.buffindex);

        let mut commentstopoffset: usize = (<usize as TryInto<isize>>::try_into(comment.content.len()).unwrap() + min((*comment.reverseindexraw.borrow_mut()).try_into().unwrap(), 0)).try_into().unwrap(); // comment.reverseindex is negative when its out of bounds 
        let mut commentstartoffset: usize = *comment.frontoffsetraw.borrow_mut(); // Todo: explain this better

        let commentfillength: usize = commentstopoffset - commentstartoffset;

        self.displaybuffer[self.buffindex..(self.buffindex + commentfillength)].copy_from_slice(&comment.content[commentstartoffset..commentstopoffset]);//&comment.content[commentstartoffset..commentstopoffset]);

        self.buffindex += commentfillength;//commentfillength;
        
        let charlength: usize = *comment.rearoffset.borrow_mut() - *comment.frontoffset.borrow_mut();

        if *comment.rearoffset.borrow_mut() > 0 {
            *comment.rearoffset.borrow_mut() -= 1;
            while comment.content.len() < 0 && comment.content[<isize as TryInto<usize>>::try_into(<usize as TryInto<isize>>::try_into(comment.content.len() - 1).unwrap() + (*comment.reverseindexraw.borrow_mut())).unwrap()] >> 6 == 0b10 {
                *comment.reverseindexraw.borrow_mut() += 1;
            }
            
            offset = -<usize as TryInto<isize>>::try_into((*comment.rearoffset.borrow_mut()).try_into().unwrap()).unwrap();
            
        } else {
            offset = (*comment.reverseindexraw.borrow_mut()).try_into().unwrap();
        }

        if comment.width as isize + offset > x.try_into().unwrap() {
            *comment.frontoffset.borrow_mut() += 1;
            while comment.content[*comment.frontoffsetraw.borrow_mut()] >> 6 == 0b10 {
                *comment.frontoffsetraw.borrow_mut() += 1;
            } 
        }
         
        *comment.reverseindexraw.borrow_mut() += 1;
        return charlength;
    }

    //fn edgecomment2buff(&mut self, ) -> usize {
        
    //}

    fn changecolor(&mut self, color: u8, index: usize) {
        const ANSICOLOR: [u8; 5] = [b'\x1B', b'[', b'3', b'1', b'm' ];
        self.displaybuffer[index..index + 5].copy_from_slice(&ANSICOLOR);
        self.displaybuffer[index + 3] = color;
        self.buffindex += 6;
    }

    fn addcomment(&mut self, lineindex: usize, comment: Comment) {
        // adds item to the start of the list and moves ever other item by 1 so the list size
        // increases by 1
        let line = &mut (self.activecomments[lineindex]);
        let mut commentsindex: usize = line.len() - 1;

        line.push(Rc::clone(&line[commentsindex]));
        
        while commentsindex > 1 {
            commentsindex -= 1;
            line[commentsindex + 1] = Rc::clone(&line[commentsindex]);
        }

        line[0] = Rc::new(comment);
    }

    fn init(&mut self, x: usize, y: usize) {
        self.xsize = x;
        self.ysize = y;
        let mut newcomment: Comment;
        let mut newline: Vec<Rc<Comment>>;

        while self.activecomments.len() < y {
            newline = vec![];

            newcomment = Comment{
                content: Vec::new(),
                width: 0, color: b'1',
                rearoffset: RefCell::new(0),
                reverseindexraw: RefCell::new(0),
                frontoffset: RefCell::new(0),
                frontoffsetraw: RefCell::new(0),
            };

            newline.push(Rc::new(newcomment));

            self.activecomments.push(newline);
        }
        while self.activecomments.len() > y {
            self.activecomments.pop();
        }

        while self.displaybuffer.len() < (self.ysize * 2 * self.xsize) {
            self.displaybuffer.push(b' ');
        }
    }
}
 
fn curl_filter_comment (board: &str, page: u8) -> Vec<Comment> {
    const GREATERTHAN: [u8; 3] = ['g' as u8, 't' as u8, ';' as u8,];
    const APOSTROPHE: [u8; 5] = ['#' as u8, '0' as u8, '3' as u8, '9' as u8, ';' as u8,];
    const QUOTATION: [u8; 5] = ['q' as u8, 'u' as u8, 'o' as u8, 't' as u8, ';' as u8,];
    let data: Vec<u8> = f4chanrequester(board, page);
    let mut comments: Vec<Comment> = vec![];
    let mut colorcounter: u8 = 0;
    let mut dataindex: usize = 0;
    let mut commentwidth: usize;
    let mut commentcontent: Vec<u8>;
    let mut commentlength: usize;


    while dataindex < (data.len() - 30) {
        if commentstarted(&data[dataindex..dataindex + 5]) { 
            
            commentwidth = 0;

            commentcontent = Vec::new();

            dataindex += 7;
            while !(data[dataindex] == b'"') {
                if data[dataindex] == (b'<') {
                    while data[dataindex] != (b'>') {
                        dataindex += 1;
                    }
                    //comments[commentindex].push_str("  ");
                } else if data[dataindex] == b'&' {
                    dataindex += 1;
                    if textcheck(&data[dataindex..dataindex + 3], &GREATERTHAN) {

                        commentcontent.push(b' ');
                        commentcontent.push(b'>');
                        commentcontent.push(b' ');


                        commentwidth += 2;
                        dataindex += 2;
                    } else if textcheck(&data[dataindex..dataindex + 5], &APOSTROPHE) {

                        commentcontent.push(b'\'');

                        dataindex += 4;
                    } else if textcheck(&data[dataindex..dataindex + 5], &QUOTATION) {

                        commentcontent.push(b'"');

                        dataindex += 4;
                    }
                } else if data[dataindex] != b'\\' {

                    commentcontent.push(data[dataindex]);

                }
                dataindex += 1;
                while (data[dataindex] >> 6) == 0b10 {
                    commentcontent.push(data[dataindex]);
                    dataindex += 1;
                }

                commentwidth += 1;

            }
            dataindex += 20;

            commentlength = commentcontent.len();

            comments.push(Comment{
                color: (49 + colorcounter) as u8,
                content: commentcontent,
                width: commentwidth,
                rearoffset: RefCell::new(commentwidth - 1),
                reverseindexraw: RefCell::new(-(<usize as TryInto<isize>>::try_into(commentlength - 1).unwrap())),
                frontoffset: RefCell::new(0),
                frontoffsetraw: RefCell::new(0),
            });

            colorcounter = 1 + (colorcounter + 1) % 9;
        }
        dataindex += 1;
    }
    return comments;
}


fn main() {
    const SCREENCLEAR: [u8; 4] = [b'\x1B', b'[', b'2', b'J'];
    let board: String = open();

    let mut _page: u8 = 1;
    let mut _activestack: usize = 1;
    let mut comment_vault: [Vec<Comment>; 2] = [
        curl_filter_comment(&board, _page),
        Default::default()
    ];

    let mut textlength: usize; // this is a necessarry value because ansi color changes require unseen bytes
    let mut linelength: usize;

    let mut display: Displayfield = Displayfield {
        activecomments: Default::default(),
        displaybuffer: Default::default(),
        buffindex: 0,
        xsize: 0,
        ysize: 0,
    };

    display.init(238, 59);

    let mut linecommentindex: usize;
    let mut commentfill: usize;

    loop {
        display.buffindex = 0;
        display.displaybuffer[0..4].copy_from_slice(&SCREENCLEAR);
        display.buffindex += 5;
        textlength = 0;

        for lineindex in 0..display.ysize - 1 {

            display.displaybuffer[display.buffindex] = b'\n';

            display.buffindex += 1;

            linecommentindex = display.activecomments[lineindex].len();

            if *display.activecomments[lineindex][linecommentindex - 1].frontoffset.borrow_mut() > display.activecomments[lineindex][linecommentindex - 1].width {
                display.activecomments[lineindex].pop();

                linecommentindex -= 1;
            }

            linelength = 0;

            while linecommentindex > 0 && linelength < display.xsize {

                linecommentindex -= 1;

                commentfill = display.comment2buff(Rc::clone(&display.activecomments[lineindex][linecommentindex]));
                textlength += commentfill;
                linelength += commentfill;
            }

            // check if theres a leftover byte and if there is a new comment is added
            if linelength < display.xsize {
                if comment_vault[_activestack].is_empty() {
                    _page = 1 + (_page % 8);
                    comment_vault[_activestack] = curl_filter_comment(&board, _page);
                }
                display.addcomment(lineindex, comment_vault[_activestack].pop().expect("Somehow no comment found (input to addcomment)"));

                display.changecolor(display.activecomments[lineindex][0].color, display.buffindex);
                display.displaybuffer[display.buffindex] = display.activecomments[lineindex][0].content[0];

                display.buffindex += 1;
            }
        }

        print!("{}", std::str::from_utf8(&display.displaybuffer[0..display.buffindex]).unwrap());

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
