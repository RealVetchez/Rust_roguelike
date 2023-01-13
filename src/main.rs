use tcod::colors;
use tcod::colors::*;
use tcod::console::*;
use core::num;
use std::cmp;
use rand::Rng;
use tcod::map::{FovAlgorithm, Map as FovMap};


//screen width
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20;

//map size
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;

const PLAYER: usize = 0;

//colors
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

//FOV
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // light walls or not
const TORCH_RADIUS: i32 = 10;



// TYPES
type Map = Vec<Vec<Tile>>;


// ENUMS

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Debug, PartialEq)]
enum AI {
    Basic,
}

// STRUCTS

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

struct Game {
    map: Map,
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    x2: i32,
    y1: i32,
    y2: i32,
}

// combat related structs and methods
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
}

// generic object for item on screen 
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<AI>,
}

//struct for tiles
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}

// IMPLs

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            explored: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false,
        }
    }
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object { 
            x: x, 
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
        }
    }

    //move object if the destination is not blocked



    // set the color and draw the character that represents this object
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;

        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h:i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        //returns true if intersects with another rectangle
        (self.x1 <= other.x2)
        && (self.x2 >= other.x1)
        && (self.y1 <= other.y2)
        && (self.y2 >= other.y1)
    }
}

fn main() {
    //initialize the window
    let root: Root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Roguelike in libtcod")
        .init();    


    let mut tcod = Tcod { 
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };


    //limit the fps of the window
    tcod::system::set_fps(LIMIT_FPS);

    //create player object
    let mut player = Object::new(0, 0, '@',"player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
    });


    //list of objects
    let mut objects = vec![player];

    let mut game = Game {
        map: make_map(&mut objects),
    };

    //populate the map tiles for FOV usage
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            )
        }
    }

    //force FOV 'recompute' for first time through the game loop
    let mut previous_player_position = (-1, -1);

    //main game loop
    while !tcod.root.window_closed() {
        //clear previous frame
        tcod.con.clear();
        

        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();

        let player = &mut objects[PLAYER];

        previous_player_position = objects[PLAYER].pos();

        let player_action = handle_keys(&mut tcod, &game, &mut objects);
        if player_action == PlayerAction::Exit {
            break;
        }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &game, &mut objects);
                }
            }
        }
    }
}

// key input handler
fn handle_keys(tcod: &mut Tcod, game: &Game, objects: &mut Vec<Object>) -> PlayerAction {
    use PlayerAction::*; 
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match (key, key.text(), player_alive) {(
        Key { 
            code: Enter, 
            alt: true, 
            .. 
        },
        _,
        _,
     ) => {
            // alt + enter toggles fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            return DidntTakeTurn;
        }
        (Key { code: Escape, .. }, _, _) => return Exit,
        (Key {code: Up, ..}, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key {code: Down, ..}, _, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        (Key {code: Left, ..}, _, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key {code: Right, ..}, _, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }

        _ => return DidntTakeTurn
    }
}


fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let color = match (visible, wall) {
                //not visible
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                //visible
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            //set visible tiles to explored
            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                *explored = true;
            } 
            if *explored {
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }
    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
    }
    blit(
    &tcod.con,
    (0,0),
    (MAP_WIDTH, MAP_HEIGHT),
    &mut tcod.root,
    (0,0),
    1.0,
    1.0,
    );
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];
    
    for _ in 0..MAX_ROOMS {
        //random width and height 
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        
        //randomize position without going out of bounds
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);
        //run through other rooms and see if they intersect
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));
        
        if !failed {
            //no intersections, room is valid
            // 'paint' it to the map tiles
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() 
            {
                //check if this is the first room
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                //all rooms after the first room
                //connect to previous rooms with a create_h_tunnel

                //center coordinates of previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                //toss a coin (generates random bool value)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    //vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }
    

    map
}

fn create_room(room: Rect, map: &mut Map) {
    //go through tiles in the rectangle and make them passable
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y:i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y1)..(cmp::max(y1,y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        //choose a spawn position
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let mut monster = if rand::random::<f32>() < 0.8 {  //80% chance
            //create monster
            let mut orc = Object::new(x, y, 'o',"orc", colors::DESATURATED_GREEN, true);
            orc.fighter = Some(Fighter {
                max_hp: 10,
                hp: 10,
                defense: 0,
                power: 3,
            });
            orc.ai = Some(AI::Basic);
            orc
        } else {
            let mut troll = Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true);
            troll.fighter = Some(Fighter {
                max_hp: 16,
                hp: 16,
                defense: 1,
                power: 4,
            });
            troll.ai = Some(AI::Basic);
            troll
        };
        monster.alive = true;
        objects.push(monster);
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }

    objects.iter().any(|object| object.blocks &&object.pos() == (x,y))
}

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x +dx, y+dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn player_move_or_attack(dx: i32, dy: i32, game: &Game, objects: &mut [Object]) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    //try to find attackable object
    let target_id = objects.iter().position(|object| object.pos() ==(x, y));

    //attack if target found, otherwise move
    match target_id {
        Some(target_id) => {
            println!("The {} laughs at your puny attempts to attack him", objects[target_id].name);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    //normalize it to length 1, then round it and convert to integer so the movement fits the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &Game, objects: &mut [Object]) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            //move towards player if far 
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            //close enough to attack
            let monster = &objects[monster_id];
            println!("The attack of the {} bounces off your armor", monster.name);
        }
    }
}