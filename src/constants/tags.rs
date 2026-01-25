/// Maximum number of tags a user can select
pub const MAX_TAGS: usize = 4;

/// Available tags for casual voice channels
pub const CASUAL_TAGS: &[&str] = &[
    "Gaming",
    "Music",
    "Movies",
    "Tech",
    "Sports",
    "Art",
    "Chill",
    "Study",
    "Work",
    "Languages",
    "Anime",
    "Books",
    "Cooking",
    "Fitness",
    "Travel",
];

/// Available tags for debate voice channels
pub const DEBATE_TAGS: &[&str] = &[
    "Politics",
    "Philosophy",
    "Science",
    "Religion",
    "Ethics",
    "Economics",
    "History",
    "Law",
    "Technology",
    "Environment",
    "Education",
    "Healthcare",
    "Society",
    "Psychology",
    "Culture",
];

/// Get tags for a channel type
pub fn get_tags(casual: bool) -> &'static [&'static str] {
    if casual {
        CASUAL_TAGS
    } else {
        DEBATE_TAGS
    }
}
