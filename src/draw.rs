use crate::data_model::{
    Game, PIECE_GRID_HEIGHT, PIECE_GRID_WIDTH, Player, WALL_GRID_WIDTH, WallOrientation,
};
use ggez::graphics::{self, PxScale, TextFragment, Transform};
use ggez::mint::{Point2, Vector2};
use ggez::{Context, GameResult};

enum Color {
    PlayerA,
    PlayerB,
    PieceSquare,
    Wall,
    Background,
    Text,
}

impl Color {
    fn to_ggez_color(&self) -> graphics::Color {
        match self {
            Color::PlayerA => graphics::Color::from_rgb(248, 248, 248),
            Color::PlayerB => graphics::Color::from_rgb(38, 38, 38),
            Color::Wall => graphics::Color::from_rgb(86, 83, 82),
            Color::PieceSquare => graphics::Color::from_rgb(240, 217, 181),
            Color::Background => graphics::Color::from_rgb(181, 136, 99),
            Color::Text => graphics::Color::from_rgb(255, 255, 255),
        }
    }
}

pub fn draw(game: &Game, ctx: &mut Context) -> GameResult {
    let window_size = ctx.gfx.window().inner_size();
    let total_board_size = u32::min(window_size.width, window_size.height) as f32;
    const PIECE_SQUARE_SIZE_TO_WALL_WIDTH_RATIO: f32 = 5.0;
    let wall_thickness = total_board_size
        / (PIECE_GRID_WIDTH as f32 * PIECE_SQUARE_SIZE_TO_WALL_WIDTH_RATIO
            + WALL_GRID_WIDTH as f32);
    let piece_square_size = PIECE_SQUARE_SIZE_TO_WALL_WIDTH_RATIO * wall_thickness;
    let wall_length = 2.0 * piece_square_size + wall_thickness;
    let piece_radius = piece_square_size / 3.0;
    let mut canvas = graphics::Canvas::from_frame(ctx, Color::Background.to_ggez_color());
    for x in 0..PIECE_GRID_WIDTH {
        for y in 0..PIECE_GRID_HEIGHT {
            let screen_x = x as f32 * (piece_square_size + wall_thickness);
            let screen_y = y as f32 * (piece_square_size + wall_thickness);
            let rect =
                graphics::Rect::new(screen_x, screen_y, piece_square_size, piece_square_size);
            canvas.draw(
                &graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::fill(),
                    rect,
                    Color::PieceSquare.to_ggez_color(),
                )?,
                graphics::DrawParam::default(),
            );
        }
    }
    for (i, piece_position) in game.board.player_positions.iter().enumerate() {
        let point = [
            piece_position.x as f32 * (piece_square_size + wall_thickness)
                + piece_square_size / 2.0,
            piece_position.y as f32 * (piece_square_size + wall_thickness)
                + piece_square_size / 2.0,
        ];
        let color = if i == Player::White.as_index() {
            Color::PlayerA
        } else {
            Color::PlayerB
        }
        .to_ggez_color();
        canvas.draw(
            &graphics::Mesh::new_circle(
                ctx,
                graphics::DrawMode::fill(),
                point,
                piece_radius,
                0.1,
                color,
            )?,
            graphics::DrawParam::default(),
        );
    }
    for (x, col) in game.board.walls.0.iter().enumerate() {
        for (y, wall) in col.iter().enumerate() {
            let screen_x = x as f32 * (piece_square_size + wall_thickness) + piece_square_size;
            let screen_y = y as f32 * (piece_square_size + wall_thickness) + piece_square_size;
            if let Some(wall) = wall {
                let rect = match wall {
                    WallOrientation::Horizontal => graphics::Rect::new(
                        screen_x - piece_square_size,
                        screen_y,
                        wall_length,
                        wall_thickness,
                    ),
                    WallOrientation::Vertical => graphics::Rect::new(
                        screen_x,
                        screen_y - piece_square_size,
                        wall_thickness,
                        wall_length,
                    ),
                };
                canvas.draw(
                    &graphics::Mesh::new_rectangle(
                        ctx,
                        graphics::DrawMode::fill(),
                        rect,
                        Color::Wall.to_ggez_color(),
                    )?,
                    graphics::DrawParam::default(),
                );
            } else {
                canvas.draw(
                    &graphics::Text::new(TextFragment {
                        text: format!("{x}{y}"),
                        color: Some(Color::Text.to_ggez_color()),
                        font: Some("LiberationMono-Regular".into()),
                        scale: Some(PxScale::from(wall_thickness)),
                    }),
                    graphics::DrawParam {
                        transform: Transform::Values {
                            dest: Point2 {
                                x: screen_x,
                                y: screen_y,
                            },
                            offset: Point2 { x: 0.0, y: 0.0 },
                            rotation: 0.0,
                            scale: Vector2 { x: 1.0, y: 1.0 },
                        },
                        ..Default::default()
                    },
                )
            }
        }
    }
    canvas.finish(ctx)
}
