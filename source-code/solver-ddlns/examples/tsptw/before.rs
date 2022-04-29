use papier_lns::Matrix;

use crate::{BitSet256, tsptw::TimeWindow};


#[derive(Debug, Clone)]
pub struct Before {
    pred: Vec<BitSet256>
}
impl Before {
    pub fn new(n: usize, dist: &Matrix<usize>, tw: &[TimeWindow]) -> Self {
        let mut pred = vec![BitSet256::empty(); n];
        for (i, pred_i) in pred.iter_mut().enumerate() {
            for j in 0..n {
                if i == j {
                    continue;
                } else {
                    let dist = dist[(i, j)];
                    let arr  = tw[i].start.saturating_add(dist);
                    let end  = tw[j].stop;
                    if arr > end {
                        pred_i.add(j);
                    }
                }
            }
        }
        Self{pred}
    }
    /// must x be before y ?
    pub fn is_before(&self, x: usize, y: usize) -> bool {
        self.pred[y].contains(x)
    }
    /// is there any city in bs that must be visited before x ?
    pub fn any_before(&self, bs: BitSet256, x: usize) -> bool {
        self.pred[x].intersect(&bs)
    }
}