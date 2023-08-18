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
    let mut grid_coords = (0..8).zip(std::array::from_fn::<_, 8, _>(|_| 0..8)).fold(
        Vec::new(),
        |mut acc, (x, y_range)| {
            y_range.for_each(|y| acc.push((x, y)));
            acc
        },
    );

    loop {
        mq::clear_background(mq::BLACK);

        sort(&mut grid_coords);
        draw_gridcells(&grid_coords);
        draw_ids(&grid_coords);
        draw_lines(&mut grid_coords);
        draw_highlights(&mut grid_coords);

        mq::next_frame().await
    }
}

///Draw a w*w rectangle with 10% padding, and the coord and id of the gridcell
fn draw_gridcell(x: u32, y: u32, w: f32, padding: f32, color: mq::Color) {
    mq::draw_rectangle(
        x as f32 * (w + padding),
        y as f32 * (w + padding),
        w,
        w,
        color,
    );
    mq::draw_text(
        &format!("({}, {})", x, y),
        x as f32 * (w + padding) + padding,
        y as f32 * (w + padding) + padding * 2.0,
        16.0,
        mq::WHITE,
    );
}

fn draw_id(x: u32, y: u32, w: f32, padding: f32, id: u64) {
    mq::draw_text(
        &format!("{}", id),
        x as f32 * (w + padding) + padding,
        y as f32 * (w + padding) + padding * 4.0,
        16.0,
        mq::WHITE,
    );
}

fn draw_gridcells(grid_coords: &Vec<(u32, u32)>) {
    for (x, y) in grid_coords.iter() {
        draw_gridcell(*x, *y, 60.0, 6.0, mq::BLUE);
    }
}

fn draw_ids(grid_coords: &Vec<(u32, u32)>) {
    for (i, (x, y)) in grid_coords.iter().enumerate() {
        draw_id(*x, *y, 60.0, 6.0, i as u64);
    }
}

fn sort(grid_coords: &mut Vec<(u32, u32)>) {
    grid_coords.sort_by(|(x1, y1), (x2, y2)| {
        morton_encode([*x1, *y1]).cmp(&morton_encode([*x2, *y2]))
    });
}

///Draw a line going from one gridcell to the next, in the order they are stored in the vector
fn draw_lines(grid_coords: &Vec<(u32, u32)>) {
    let mut iter = grid_coords.iter().peekable();
    while let Some((x, y)) = iter.next() {
        if let Some((next_x, next_y)) = iter.peek() {
            mq::draw_line(
                *x as f32 * 66.0 + 33.0,
                *y as f32 * 66.0 + 33.0,
                *next_x as f32 * 66.0 + 33.0,
                *next_y as f32 * 66.0 + 33.0,
                2.0,
                mq::RED,
            );
        }
    }
}

//highlight the gridcell highlighted by the mouse, and the 3 gridcells that are next to it
fn draw_highlights(grid_coords: &Vec<(u32, u32)>) {
    let (mx, my) = mq::mouse_position();
    let w = 60.0;
    let offset = 6.0;
    let mut iter = grid_coords.iter().enumerate();
    while let Some((n, (x, y))) = iter.next() {
        let contains_mouse = mx >= *x as f32 * (w + offset)
            && mx <= *x as f32 * (w + offset) + w
            && my >= *y as f32 * (w + offset)
            && my <= *y as f32 * (w + offset) + w;
        //highlight the gridcell and the <x> either side of it
        if contains_mouse {
            let from = if n < 4 { 0 } else { n - 4 };
            let to = n+4;
            for i in from..=to {
                if let Some((x, y)) = grid_coords.get(i) {
                    draw_gridcell(
                        *x,
                        *y,
                        w,
                        offset,
                        mq::Color {
                            r: 0.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.5,
                        },
                    );
                }
            }
        }
    }
}
