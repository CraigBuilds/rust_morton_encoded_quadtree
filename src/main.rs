use macroquad::prelude as mq;
use morton_encoding::morton_encode;

//TODO integer offset didn't work, but bitwise ops might...
//Each quadrant of the morton quadtree shares the most significant bits.
//i.e in the main top left quad, all leaves inside it start with 00,
//the top right quad, all leaves start with 01, bottom left 10, bottom right 11.
//This pattern continues for nested quads.
//So we can get the parent quad of the leaf by looking at these bits.
//However, this doesn't encode a relationship between coords close to each other at quad boundaries.
//In the center of the main quad, a bitwise inverse of a leaf gives the coord of the closest opposite leaf in the other quad.
//For subquads, its a bitwise inverse of the least 4 significant bits.
//note: in this example we only need 6 bits for out morton codes
const W: f32 = 60.0;
const PADDING: f32 = 6.0;
const DIM : usize = 8;

fn main() {
    macroquad::Window::from_config(
        mq::Conf {
            window_title: "Morton Encoding".to_owned(),
            ..Default::default()
        },
        amain(),
    );
}

async fn amain() {
    //construct the coords in row major order
    
    let mut grid_coords = (0..DIM).zip(std::array::from_fn::<_, DIM, _>(|_| 0..DIM)).fold(
        Vec::new(),
        |mut acc, (x, y_range)| {
            y_range.for_each(|y| acc.push((x as u8, y as u8)));
            acc
        },
    );

    let mut method = NearestNeighboursStratergy::SharesBits(BitMask::WholeGrid);
    let mut method = NearestNeighboursStratergy::SharesNoBits;

    loop {
        mq::clear_background(mq::BLACK);

        if mq::is_key_pressed(mq::KeyCode::Space) {
            method = match method {
                NearestNeighboursStratergy::SharesBits(BitMask::WholeGrid) => {
                    NearestNeighboursStratergy::SharesBits(BitMask::TopLevelParent)
                }
                NearestNeighboursStratergy::SharesBits(BitMask::TopLevelParent) => {
                    NearestNeighboursStratergy::SharesBits(BitMask::SecondLevelParent)
                }
                NearestNeighboursStratergy::SharesBits(BitMask::SecondLevelParent) => {
                    NearestNeighboursStratergy::SharedParents(BitMask::WholeGrid)
                }
                NearestNeighboursStratergy::SharedParents(BitMask::WholeGrid) => {
                    NearestNeighboursStratergy::SharesBits(BitMask::WholeGrid)
                }
                _ => panic!("Invalid transition")
            };
        }
        sort(&mut grid_coords);
        draw_gridcells(&grid_coords);
        draw_ids(&grid_coords);
        // draw_lines(&mut grid_coords);
        draw_highlights(&mut grid_coords, &method);

        mq::next_frame().await
    }
}

///Draw a w*w rectangle with 10% padding, and the coord and id of the gridcell
fn draw_gridcell(x: u8, y: u8, color: mq::Color) {
    mq::draw_rectangle(
        x as f32 * (W + PADDING),
        y as f32 * (W + PADDING),
        W,
        W,
        color,
    );
    mq::draw_text(
        &format!("({}, {})", x, y),
        x as f32 * (W + PADDING) + PADDING,
        y as f32 * (W + PADDING) + PADDING * 2.0,
        16.0,
        mq::WHITE,
    );
}

fn draw_text(x: u8, y: u8, text: &str, v_paddings: f32) {
    mq::draw_text(
        &text,
        x as f32 * (W + PADDING) + PADDING,
        y as f32 * (W + PADDING) + (PADDING * v_paddings),
        14.0,
        mq::WHITE,
    );
}

fn draw_gridcells(grid_coords: &Vec<(u8, u8)>) {
    for (x, y) in grid_coords.iter() {
        draw_gridcell(*x, *y, mq::BLUE);
    }
}

fn draw_ids(grid_coords: &Vec<(u8, u8)>) {
    for (i, (x, y)) in grid_coords.iter().enumerate() {
        draw_text(*x, *y, &format!("{}", i), 4.0);
        draw_text(*x, *y, &format!("lid:{:#04b}", morton_encode([*x, *y]) & 0b11), 6.0);
        draw_text(*x,*y,&format!("qid:{:#04b}", (morton_encode([*x, *y]) >> 2) & 0b11), 10.0);
        draw_text(*x,*y,&format!("Qid:{:#04b}", (morton_encode([*x, *y]) >> 4) & 0b11), 8.0);
    }
}

fn sort(grid_coords: &mut Vec<(u8, u8)>) {
    grid_coords
        .sort_by(|(x1, y1), (x2, y2)| morton_encode([*x1, *y1]).cmp(&morton_encode([*x2, *y2])));
}

///Draw a line going from one gridcell to the next, in the order they are stored in the vector
#[allow(dead_code)]
fn draw_lines(grid_coords: &Vec<(u8, u8)>) {
    let mut iter = grid_coords.iter().peekable();
    while let Some((x, y)) = iter.next() {
        if let Some((next_x, next_y)) = iter.peek() {
            let dist = W + PADDING;
            mq::draw_line(
                *x as f32 * dist + (dist / 2.0),
                *y as f32 * dist + (dist / 2.0),
                *next_x as f32 * dist + (dist / 2.0),
                *next_y as f32 * dist + (dist / 2.0),
                2.0,
                mq::RED,
            );
        }
    }
}

enum NearestNeighboursStratergy {
    SharesBits(BitMask),
    SharedParents(BitMask),
    SharesNoBits
}

//WARNING: This currently only works for DIM=8
#[derive(Clone, Copy)]
enum BitMask {
    WholeGrid = 8,
    TopLevelParent = 4,
    SecondLevelParent = 2
}

impl NearestNeighboursStratergy {
    fn highlight(&self, grid_coords: &Vec<(u8, u8)>, n: usize) {
        match self {
            NearestNeighboursStratergy::SharesBits(mask) => {
                self.shares_bits(grid_coords, n, mask);
            }
            NearestNeighboursStratergy::SharedParents(mask) => {
                self.shared_parents(grid_coords, n, mask);
            }
            NearestNeighboursStratergy::SharesNoBits => {
                self.shares_no_bits(grid_coords, n);
            }
        }
    }

    ///Highlights the sqaures if they share the same bits after the mask
    ///All leafs in the morton quadtree share the same 2 most significant bits (0,0,x,x,x,x,x,x)
    ///All cells within each of the 4 main quads share the next 2 bits (0,0,_,_,x,x,x,x)
    ///  (i.e top left is 00, top right is 01, bottom left is 10, bottom right is 11)
    ///All subquads within the main quads share the next 2 bits (0,0,x,x,_,_,x,x)
    ///  (i.e top left is 00, top right is 01, bottom left is 10, bottom right is 11)
    ///All leaves within each subquad share the next 2 bits (lsbs) (0,0,x,x,x,x,_,_)
    fn shares_bits(&self, grid_coords: &Vec<(u8, u8)>, n: usize, mask: &BitMask) {
        let shift = *mask as usize;
        let this_coords = grid_coords.get(n).unwrap();
        let this_m_code = morton_encode([this_coords.0, this_coords.1]);
        let this_parent = (this_m_code >> shift) & 0b11;
        for (x, y) in grid_coords {
            let other_mcode = morton_encode([*x, *y]);
            let other_parent = (other_mcode >> shift) & 0b11;
            if other_parent == this_parent {
                draw_gridcell(*x, *y, mq::Color {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.5,
                });
            }
        }
    }
    ///Highlights the sqaures if they share no bits at all in common
    fn shares_no_bits(&self, grid_coords: &Vec<(u8, u8)>, n: usize) {
        let this_coords = grid_coords.get(n).unwrap();
        let this_m_mcode = morton_encode([this_coords.0, this_coords.1]);
        for (x, y) in grid_coords {
            let other_mcode = morton_encode([*x, *y]);
            if other_mcode & this_m_mcode == 0 {
                draw_gridcell(*x, *y, mq::Color {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.5,
                });
            }
        }
    }
    /// Highlights the sqaures that share the same parent.
    /// It finds all leaves that share the same bits after the mask, and also share bits after the mask-2, mask-4 etc..
    fn shared_parents(&self, grid_coords: &Vec<(u8, u8)>, n: usize, mask: &BitMask) {
        let shift = *mask as usize;
        let this_coords = grid_coords.get(n).unwrap();
        let this_m_code = morton_encode([this_coords.0, this_coords.1]);
        let mut this_parents = Vec::new();
        let mut current_shift = shift;
        while current_shift >= 2 {
            let parent = (this_m_code >> current_shift) & 0b11;
            this_parents.push(parent);
            current_shift -= 2;
        }
        for (x, y) in grid_coords {
            let other_mcode = morton_encode([*x, *y]);
            let mut other_parents = Vec::new();
            let mut current_shift = shift;
            while current_shift >= 2 {
                let parent = (other_mcode >> current_shift) & 0b11;
                other_parents.push(parent);
                current_shift -= 2;
            }
            if this_parents.iter().zip(other_parents.iter()).all(|(a, b)| a == b) {
                draw_gridcell(*x, *y, mq::Color {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.5,
                });
            }
        }
    }
}

//highlight the gridcell highlighted by the mouse, and the 3 gridcells that are next to it
fn draw_highlights(grid_coords: &Vec<(u8, u8)>, method: &NearestNeighboursStratergy) {
    let (mx, my) = mq::mouse_position();
    let mut iter = grid_coords.iter().enumerate();
    while let Some((n, (x, y))) = iter.next() {
        let contains_mouse = mx >= *x as f32 * (W + PADDING)
            && mx <= *x as f32 * (W + PADDING) + W
            && my >= *y as f32 * (W + PADDING)
            && my <= *y as f32 * (W + PADDING) + W;
        //highlight the gridcell and the <x> either side of it
        if contains_mouse {
            method.highlight(grid_coords, n);
        }
    }
}
