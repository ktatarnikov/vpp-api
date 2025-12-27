use crate::api_gen::file_schema::VppJsApiFile;

#[derive(Debug, Clone)]
pub struct ImportsFiles {
    pub name: String,
    pub file: Box<VppJsApiFile>,
}

pub fn merge(
    mut arr: Vec<ImportsFiles>,
    left: usize,
    mid: usize,
    right: usize,
) -> Vec<ImportsFiles> {
    let n1 = mid - left;
    let n2 = right - mid;
    let l1 = arr.clone();
    let r1 = arr.clone();
    let l = &l1[left..mid];
    let r = &r1[mid..right];
    /* Merge the temp arrays back into arr[l..r]*/
    let mut i = 0; // Initial index of first subarray
    let mut j = 0; // Initial index of second subarray
    let mut k = left; // Initial index of merged subarray
    while i < n1 && j < n2 {
        if l[i].file.imports.len() < r[j].file.imports.len() {
            arr[k] = l[i].clone();
            i = i + 1;
        } else {
            arr[k] = r[j].clone();
            j = j + 1;
        }
        k = k + 1;
    }
    while i < n1 {
        arr[k] = l[i].clone();
        i = i + 1;
        k = k + 1;
    }
    /* Copy the remaining elements of R[], if there
    are any */
    while j < n2 {
        arr[k] = r[j].clone();
        j = j + 1;
        k = k + 1;
    }
    arr
}
// Performing Merge Sort According to import lenght
pub fn merge_sort(mut arr: Vec<ImportsFiles>, left: usize, right: usize) -> Vec<ImportsFiles> {
    if right - 1 > left {
        let mid = left + (right - left) / 2;
        arr = merge_sort(arr, left, mid);
        arr = merge_sort(arr, mid, right);
        arr = merge(arr, left, mid, right);
    }
    arr
}
