pub fn arr_top_n<T>(ts: &[T], n: usize) -> Option<&T> {
    let ind = ts.len() as i32 - (n as i32) - 1;
    if ind < 0 {
        None
    } else {
        ts.get(ind as usize)
    }
}
