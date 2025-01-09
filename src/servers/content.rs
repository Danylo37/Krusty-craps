use std::collections::HashMap;
use rand::random;

pub fn choose_random_texts() -> Vec<(u8, String)>{
    let trying_closures = |x:u8| {
        if x < 4 {
            return x+3
        }else{
            x
        }
    };

    let n_files = trying_closures(random::<u8>()%10);

    let mut vec_files = Vec::new();
    if(random::<u8>()%2 == 0){
        for i in 0..n_files{
            vec_files.push((i,TEXT[i].clone()));
        }
    }else{
        for i in (0..n_files).rev(){
            vec_files.push((i,TEXT[i].clone()));
        }
    }

}

pub fn get_media(vec_files: Vec<(u8, String)>) -> HashMap<String, String>{
    IMAGE_PATHS.iter().filter_map(|(index_media, ref_s, media)| {
        // Check if the `index_media` exists in `vec_files`
        if vec_files.iter().any(|(index_chose, _)| *index_chose == *index_media) {
            // Return the tuple (ref_s, media) if the condition is true
            Some((ref_s.clone(), media.clone()))
        } else {
            // Discard the element by returning None
            None
        }
    }).collect::<HashMap<String, String>>() // Collect into HashMap
}


pub const TEXT: [&str; 6] = [

    //1
    r#"Alcuni versi di Leopardi:
Ma perchè dare al sole,
Perchè reggere in vita
Chi poi di quella consolar convenga?
Se la vita è sventura,
Perchè da noi si dura?
Intatta luna, tale
E’ lo stato mortale.
Ma tu mortal non sei,
E forse del mio dir poco ti cale."#,

    //2
    "Una banana #Media[banana]",

    //3
    "Non scegliere questo testo #Media[do_not_search_this]",

    //4
    r#"Phrases by Lillo:
- a lack of belief in free will is the antidote to hate and judgement
- il disordine è tale finche non viene ordinato
- if you have to ask if you’re a member of a group, you’re probably not."#,

    //5
    r#"One of the best panoramas are next to us,
just walk up on a mountain,
sit in the middle of the forest and watch at the Sparkling snow #Media[sparkling_snow]"#,

    //6
    r#"Bigfoot Sighting Report
Location: Dense forest near Willow Creek, California
Date and Time: December 12, 2024, 4:45 PM

Image: #Media[big_foot]

Report:
While hiking along an isolated trail, approximately 5 miles from the nearest road, I encountered an unusual figure standing roughly 50 yards away in a clearing.
The figure was enormous, standing between 7 and 8 feet tall, with broad shoulders and a heavily muscled frame.
Its body appeared to be covered in dark, shaggy hair, likely black or very dark brown, and it moved with a distinct upright, bipedal gait."#,
];


pub const IMAGE_PATHS: [(u8, &str, &str); 4] =  [
    (1, "banana", "path/to/banana.jpeg"),
    (2, "do_not_search_this", "path/to/do_not_search_this.jpeg"),
    (4, "sparkling_snow", "path/to/sparkling_snow.jpeg"),
    (5, "big_foot", "path/to/big_foot.jpeg"),
];
