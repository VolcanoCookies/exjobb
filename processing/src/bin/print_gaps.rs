use std::io::Write;

use mongodb::bson::DateTime;

pub fn main() {
    let data = std::fs::read_to_string("out/gaps.txt").unwrap();

    let out_writer = std::fs::File::create("out/gaps_out.txt").unwrap();
    let mut out_writer = std::io::BufWriter::new(out_writer);

    let mut total = 0;

    data.lines().for_each(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let start = parts[0].parse::<i64>().unwrap();
        let end = parts[1].parse::<i64>().unwrap();

        let diff = end - start;
        total += diff;

        let start = DateTime::from_millis(start * 1000);
        let end = DateTime::from_millis(end * 1000);

        out_writer
            .write_fmt(format_args!(
                "{} {}\t{}\n",
                start.try_to_rfc3339_string().unwrap(),
                end.try_to_rfc3339_string().unwrap(),
                diff
            ))
            .unwrap();
    });

    println!("Total: {}", total);

    out_writer.flush().unwrap();
}
