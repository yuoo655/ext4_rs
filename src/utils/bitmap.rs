/// 检查位图中的某一位是否被设置
/// 参数 bmap: 位图数组
/// 参数 bit: 位图中的位索引
pub fn ext4_bmap_is_bit_set(bmap: &[u8], bit: u32) -> bool {
    bmap[(bit >> 3) as usize] & (1 << (bit & 7)) != 0
}

/// 检查位图中的某一位是否被清除
/// 参数 bmap: 位图数组
/// 参数 bit: 位图中的位索引
pub fn ext4_bmap_is_bit_clr(bmap: &[u8], bit: u32) -> bool {
    !ext4_bmap_is_bit_set(bmap, bit)
}

/// 设置位图中的某一位
/// 参数 bmap: 位图数组
/// 参数 bit: 位图中的位索引
pub fn ext4_bmap_bit_set(bmap: &mut [u8], bit: u32) {
    bmap[(bit >> 3) as usize] |= 1 << (bit & 7);
}

/// 清除位图中的某一位
/// 参数 bmap: 位图数组
/// 参数 bit: 位图中的位索引
pub fn ext4_bmap_bit_clr(bmap: &mut [u8], bit: u32) {
    bmap[(bit >> 3) as usize] &= !(1 << (bit & 7));
}

/// 查询位图中的空闲位
/// 参数 bmap: 位图数组
/// 参数 sbit: 起始位索引
/// 参数 ebit: 结束位索引
/// 参数 bit_id: 用于存储空闲位的索引
pub fn ext4_bmap_bit_find_clr(bmap: &[u8], sbit: u32, ebit: u32, bit_id: &mut u32) -> bool {
    let mut i: u32;
    let mut bcnt = ebit - sbit;

    i = sbit;

    while i & 7 != 0 {
        if bcnt == 0 {
            return false;
        }

        if ext4_bmap_is_bit_clr(bmap, i) {
            *bit_id = sbit;
            return true;
        }

        i += 1;
        bcnt -= 1;
    }

    let mut sbit = i;
    let mut bmap = &bmap[(sbit >> 3) as usize..];
    while bcnt >= 8 {
        if bmap[0] != 0xFF {
            for i in 0..8 {
                if ext4_bmap_is_bit_clr(bmap, i) {
                    *bit_id = sbit + i;
                    return true;
                }
            }
        }

        bmap = &bmap[1..];
        bcnt -= 8;
        sbit += 8;
    }

    for i in 0..bcnt {
        if ext4_bmap_is_bit_clr(bmap, i) {
            *bit_id = sbit + i;
            return true;
        }
    }

    false
}

/// 清除位图中的一段位
/// 参数 bmap: Mutable reference to the bitmap array.
/// 参数 start_bit: The start index of the bit range to clear.
/// 参数 end_bit: The end index of the bit range to clear.
pub fn ext4_bmap_bits_free(bmap: &mut [u8], start_bit: u32, end_bit: u32) {
    for bit in start_bit..=end_bit {
        ext4_bmap_bit_clr(bmap, bit);
    }
}