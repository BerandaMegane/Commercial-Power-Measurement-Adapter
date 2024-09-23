use core::mem::MaybeUninit;

pub struct RingBufferIndex16bit<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    head: u16,
    tail: u16,
    buf_mask: u16,
}

impl<T, const N: usize> RingBufferIndex16bit<T, N> {
    const ELEM: MaybeUninit<T> = MaybeUninit::uninit();
    const INIT: [MaybeUninit<T>; N] = [Self::ELEM; N];

    // コンストラクタ
    pub const fn new(n: usize) -> Self {
        assert!(n & (n-1) == 0, "サイズが 2 のべき乗である必要があります");
        assert!(n > 256, "サイズが 256 以下のときは RingBuffer8 を使ったほうが効率的です");

        Self {
            buffer: Self::INIT,
            head: 0,
            tail: 0,
            buf_mask: (n-1) as u16
        }
    }

    pub fn enqueue(&mut self, value: T) {
        self.buffer[self.head as usize] = MaybeUninit::new(value);
        self.head = (self.head + 1) & self.buf_mask;
    }
    
    pub fn dequeue(&mut self) -> T {
        unsafe {
            let ret: T = self.buffer[self.tail as usize].as_ptr().read(); 
            self.tail = (self.tail + 1) & self.buf_mask;
            return ret;
        }
    }

    // self の値を書き換えているので、 &mut は必要
    pub fn clear(&mut self) {
        self.tail = self.head;
    }
    
    pub fn is_empty(&self) -> bool {
        return self.tail == self.head;
    }
    
    pub fn is_full(&self) -> bool {
        return (self.head + 1) & self.buf_mask == self.tail;
    }
    
    pub fn len(&self) -> u16 {
        if self.head >= self.tail {
            return (self.head - self.tail) as u16;
        } else {
            return (self.head + self.buf_mask + 1 - self.tail) as u16;
        }
    }
}


pub struct RingBufferIndex8bit<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    head: u8,
    tail: u8,
    buf_mask: u8,
}

impl<T, const N: usize> RingBufferIndex8bit<T, N> {
    const ELEM: MaybeUninit<T> = MaybeUninit::uninit();
    const INIT: [MaybeUninit<T>; N] = [Self::ELEM; N];

    // コンストラクタ
    pub const fn new(n: usize) -> Self {
        assert!(n & (n-1) == 0, "サイズが 2 のべき乗である必要があります");
        assert!(n <= 256, "サイズが 512 超過のときは RingBuffer16 を使う必要があります");

        Self {
            buffer: Self::INIT,
            head: 0,
            tail: 0,
            buf_mask: (n-1) as u8
        }
    }

    pub fn enqueue(&mut self, value: T) {
        self.buffer[self.head as usize] = MaybeUninit::new(value);
        self.head = (self.head + 1) & self.buf_mask;
    }
    
    pub fn dequeue(&mut self) -> T {
        unsafe {
            let ret: T = self.buffer[self.tail as usize].as_ptr().read(); 
            self.tail = (self.tail + 1) & self.buf_mask;
            return ret;
        }
    }
    
    // self の値を書き換えているので、 &mut は必要
    pub fn clear(&mut self) {
        self.tail = self.head;
    }

    pub fn is_empty(&self) -> bool {
        return self.tail == self.head;
    }
    
    pub fn is_full(&self) -> bool {
        return (self.head + 1) & self.buf_mask == self.tail;
    }
    
    pub fn len(&self) -> u8 {
        if self.head >= self.tail {
            return (self.head - self.tail) as u8;
        } else {
            return (self.head + self.buf_mask + 1 - self.tail) as u8;
        }
    }
}
