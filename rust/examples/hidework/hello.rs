

struct Person {
    name: String,
    age: i32
}

impl Person {

    fn new(name: &str, age: i32) -> Person {
        Person {name: name.to_string(),
                age: age}
    }

    fn speak(&self, x: i32) {
        for _i in 0..x {
            println!("{} {}", self.name, self.age);
        }
    }
}

trait Shout {
    fn shout(&self, x:i32);
}

impl Shout for i32 {
    fn shout(&self, x:i32){
        println!("{} {}", self, x);
    }
}

fn main() {
    let s = 12;
    s.shout(0);
}