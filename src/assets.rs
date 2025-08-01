use bevy::color::Color;

// Font
pub static FONT: &str = "fonts/FiraCodeNerdFont-Regular.ttf";
pub static FONT_COLOR: Color = Color::BLACK;
pub static FONT_SIZE: f32 = 14.0;

//Images
pub static GOLD_IMAGE_ICON: &str = "ui/upkeep.png";
pub static HEART_IMAGE: &str = "ui/status_icon_life.png";
pub static BASE_TOWER: &str = "towers/base_tower.png";
pub static ENEMY_FOLDER: &str = "enemies";
pub static ALIVE_ENEMIES_ICON: &str = "enemies/Tex_creature_97_t.png";

// Colors
pub static PATH_DEBUG_COLOR: Color = Color::hsla(240.0, 0.8, 0.4, 1.0);
pub static ENEMY_COLOR: Color = Color::hsla(78.0, 0.3, 0.4, 1.0);
pub static PROJECTILE_COLOR: Color = Color::hsla(300.0, 0.4, 0.4, 1.0);
pub static HOVER_TINT_COLOR: Color = Color::hsla(0.0, 0.1, 0.1, 0.5);
pub static RANGE_INDICATOR_COLOR: Color = Color::hsla(125.0, 0.4, 0.1, 0.8);
pub static DEFAULT_HEX_COLOR: Color = Color::hsla(0.0, 1.0, 0.95, 1.0);
pub static PATH_START_COLOR: Color = Color::hsla(0.8, 0.78, 0.3, 1.0);
pub static PATH_END_COLOR: Color = Color::hsla(0.8, 0.78, 0.4, 1.0);
pub static PATH_COLOR: Color = Color::hsla(0.3, 0.5, 0.8, 1.0);

//Sounds
pub static MAIN_LOOP: &str = "music/main_loop.wav";
pub static SHOT_SOUND: &str = "music/shot.wav";

//Enemy
pub static ENEMY_RADIUS: f32 = 10.0;
pub static ENEMY_PLAYER_DAMAGE: f32 = 1.0;

//Projectile
pub static PROJECTILE_SIZE: f32 = 2.0;
pub static PROJECTILE_SPEED: f32 = 650.0;

//Player
pub static PLAYER_INITIAL_GOLD: u32 = 100;
pub static TOWER_COST: u32 = 20;
