type fn_t = fn() -> ();

fn fn1() -> () {
    println!("in fn1");
}

fn fn2() -> () {
    println!("in fn2");
}

fn str_to_fn(s: &str) -> fn_t {
    match s {
        "fn1" => fn1,
        "fn2" => fn2,
        _ => panic!("don't know that function"),
    }
}

fn fn_to_str(f: fn_t) -> & 'static str {
    match f {
        fn1 => "fn1",
        fn2 => "fn2",
        _ => panic!("do not know that one"),
    }
}

fn main() {
    let f2 = str_to_fn("fn2");
    f2();
    let f1 = str_to_fn("fn1");
    f1();

    println!("{}", fn_to_str(f2));
    println!("{}", fn_to_str(f1));


}
