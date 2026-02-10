/// AI4OSE Lab1:
///
/// 与AI合作进行操作系统内核学习的起点
fn main() {
    print!("{}", include_str!("content.txt"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn AI4OSE_Lab1_2026S() {
        assert_eq!("ai4ose".to_string() + "-lab1" + "-2026s", "ai4ose-lab1-2026s");
    }
}
