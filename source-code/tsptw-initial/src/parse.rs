use std::{fs::File, io::{BufReader, Read, BufRead, Lines}};

use array2d::Array2D;

use crate::{data::{Tsptw, TimeWindow}, error::TsptwError};

/*******************************************************************************/
/**** PARSE INSTANCE ***********************************************************/
/*******************************************************************************/

impl TryFrom<File> for Tsptw {
    type Error = TsptwError;

    fn try_from(file: File) -> Result<Self, Self::Error> {
        Tsptw::try_from(BufReader::new(file))
    }
}
impl<S: Read> TryFrom<BufReader<S>> for Tsptw {
    type Error = TsptwError;

    fn try_from(reader: BufReader<S>) -> Result<Self, Self::Error> {
        Tsptw::try_from(reader.lines())
    }
}
impl<B: BufRead> TryFrom<Lines<B>> for Tsptw {
    type Error = TsptwError;

    fn try_from(lines: Lines<B>) -> Result<Self, Self::Error> {
        let mut lc = 0;
        let mut nb_nodes = 0;
        let mut distances = Array2D::filled_with(0, nb_nodes, nb_nodes);
        let mut timewindows = vec![];

        for line in lines {
            let line = line?;
            let line = line.trim();

            // skip comment lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // First line is the number of nodes
            if lc == 0 {
                nb_nodes = line.parse::<usize>().map_err(TsptwError::NbCities)?;
                distances = Array2D::filled_with(0, nb_nodes, nb_nodes);
            }
            // The next 'nb_nodes' lines represent the distances matrix
            else if (1..=nb_nodes).contains(&lc) {
                let i = (lc - 1) as usize;
                for (j, distance) in line.split_whitespace().enumerate() {
                    let distance = distance
                        .to_string()
                        .parse::<f32>()
                        .map_err(|e| TsptwError::MatrixCoeff(i, j, e))?;
                    let distance = (distance * 10000.0) as usize;
                    distances[(i, j)] = distance;
                }
            }
            // Finally, the last 'nb_nodes' lines impose the time windows constraints
            else {
                let mut tokens = line.split_whitespace();
                let earliest = if let Some(earliest) = tokens.next() {
                    earliest.parse::<f32>().map_err(TsptwError::TwStart)?
                } else {
                    return Err(TsptwError::NoTwStart(lc));
                };

                let latest = if let Some(latest) = tokens.next() {
                    latest.parse::<f32>().map_err(TsptwError::TwStop)?
                } else {
                    return Err(TsptwError::NoTwStop(lc));
                };

                let earliest = (earliest * 10000.0) as usize;
                let latest = (latest * 10000.0) as usize;

                let timewind = TimeWindow { earliest, latest };
                timewindows.push(timewind);
            }
            lc += 1;
        }

        Ok(Tsptw {
            n_client: nb_nodes,
            dist: distances,
            tw: timewindows,
        })
    }
}
