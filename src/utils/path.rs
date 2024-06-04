use alloc::string::String;
use alloc::vec::Vec;

pub fn path_check(path: &str, is_goal: &mut bool) -> usize {
    // 遍历字符串中的每个字符及其索引
    for (i, c) in path.chars().enumerate() {
        // 检查是否到达文件名的最大长度限制
        if i >= 255 {
            break;
        }

        // 检查字符是否是路径分隔符
        if c == '/' {
            *is_goal = false;
            return i;
        }

        // 检查是否达到字符串结尾
        if c == '\0' {
            *is_goal = true;
            return i;
        }
    }

    // 如果没有找到 '/' 或 '\0'，且长度小于最大文件名长度
    *is_goal = true;
    return path.len();
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn test_ext4_path_check() {
        let mut is_goal = false;

        // 测试根路径
        assert_eq!(path_check("/", &mut is_goal), 0);
        assert!(!is_goal, "Root path should not set is_goal to true");

        // 测试普通路径
        assert_eq!(path_check("/home/user/file.txt", &mut is_goal), 0);
        assert!(!is_goal, "Normal path should not set is_goal to true");

        // 测试没有斜杠的路径
        let path = "file.txt";
        assert_eq!(path_check(path, &mut is_goal), path.len());
        assert!(is_goal, "Path without slashes should set is_goal to true");

        // 测试路径末尾的 null 字符
        let path = "home\0";
        assert_eq!(path_check(path, &mut is_goal), 4);
        assert!(
            is_goal,
            "Path with null character should set is_goal to true"
        );

        // // 测试超长文件名
        // let long_path = "a".repeat(EXT4_DIRECTORY_FILENAME_LEN + 10);
        // assert_eq!(ext4_path_check(&long_path, &mut is_goal), EXT4_DIRECTORY_FILENAME_LEN);
        // assert!(!is_goal, "Long filename should not set is_goal to true and should be truncated");
    }
}
