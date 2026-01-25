/// Profanity filter for channel names
/// Based on common profanity lists to prevent Discord TOS violations

use std::collections::HashSet;
use once_cell::sync::Lazy;

/// Common profanity/slurs that violate Discord TOS
/// This list includes English profanity - extend as needed
static BAD_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Common profanity
        "fuck", "fucking", "fucker", "fucked", "fucks", "motherfucker", "motherfucking",
        "shit", "shits", "shitty", "bullshit", "horseshit", "shitting",
        "ass", "asshole", "assholes", "asses", "dumbass", "jackass", "fatass",
        "bitch", "bitches", "bitchy", "bitching",
        "damn", "dammit", "goddamn", "goddammit",
        "hell", "hellhole",
        "crap", "crappy",
        "piss", "pissed", "pissing",
        "dick", "dicks", "dickhead", "dickwad",
        "cock", "cocks", "cocksucker", "cocksucking",
        "cunt", "cunts",
        "pussy", "pussies",
        "bastard", "bastards",
        "whore", "whores", "whorish",
        "slut", "sluts", "slutty",
        "twat", "twats",
        "wanker", "wankers", "wank",
        "bollocks", "bollock",
        "arse", "arsehole",
        "bugger",
        "bloody",
        "prick", "pricks",
        "tit", "tits", "titty", "titties",
        "boob", "boobs", "booby",

        // Slurs and hate speech (critical for TOS)
        "nigger", "nigga", "niggas", "negro", "negroes","whigga", "chigga", "whiggers", "niggers",
        "faggot", "fag", "fags", "faggots", "faggy",
        "retard", "retarded", "retards",
        "spic", "spics", "spick",
        "chink", "chinks",
        "kike", "kikes",
        "gook", "gooks",
        "wetback", "wetbacks",
        "beaner", "beaners",
        "cracker", "crackers",
        "honky", "honkey", "honkies",
        "dyke", "dykes",
        "tranny", "trannies",
        "shemale", "shemales",
        "coon", "coons",
        "jap", "japs",
        "raghead", "ragheads",
        "towelhead", "towelheads",
        "camel jockey",
        "paki", "pakis",
        "wigger", "wiggers",
        "zipperhead",
        "slope", "slopes",
        "gypsy", "gypsies",

        // Sexual content
        "porn", "porno", "pornography",
        "xxx",
        "sex", "sexual", "sexy",
        "nude", "nudes", "nudity",
        "naked",
        "horny",
        "cum", "cumming", "cumshot",
        "jizz",
        "sperm",
        "orgasm", "orgasms",
        "erection",
        "boner", "boners",
        "dildo", "dildos",
        "vibrator",
        "blowjob", "blowjobs",
        "handjob", "handjobs",
        "masturbate", "masturbation", "masturbating",
        "anal",
        "anus",
        "vagina", "vaginas", "vaginal",
        "penis", "penises",
        "clitoris", "clit",
        "labia",
        "testicle", "testicles", "testes",
        "scrotum",
        "foreskin",
        "circumcision",
        "ejaculate", "ejaculation",
        "fellatio",
        "cunnilingus",
        "sodomy",
        "incest",
        "pedophile", "pedophilia", "pedo", "paedo",
        "rape", "raping", "rapist", "raped",
        "molest", "molester", "molestation",
        "bestiality",
        "zoophile", "zoophilia",
        "necrophile", "necrophilia",

        // Violence/harmful
        "kill", "killing", "killer",
        "murder", "murderer", "murdering",
        "suicide", "suicidal",
        "terrorist", "terrorism",
        "nazi", "nazis", "nazism",
        "hitler",
        "holocaust",
        "genocide",
        "kkk", "klan",
        "jihad", "jihadist",
        "isis",
        "alqaeda", "al-qaeda",

        // Common leetspeak/evasion patterns
        "f4ck", "fvck", "phuck", "phuk",
        "sh1t", "sh!t", "s#it",
        "b1tch", "b!tch",
        "a$$", "a55",
        "d1ck", "d!ck",
        "c0ck",
        "p0rn",
        "n1gger", "n1gga", "nigg3r", "n!gger",
        "f4g", "f4gg0t",
        "r3tard", "r3t4rd",
    ]
    .into_iter()
    .collect()
});

/// Check if text contains profanity
/// Returns the first bad word found, if any
pub fn contains_profanity(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();

    // Remove common separators that might be used to evade filter
    let normalized = text_lower
        .replace(['_', '-', '.', ' ', '0', '1', '3', '4', '5', '@', '!', '$', '#'], "");

    // Check each word in the original text
    for word in text_lower.split_whitespace() {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if BAD_WORDS.contains(clean_word) {
            return Some(clean_word.to_string());
        }
    }

    // Check the normalized (no separators) version for evasion attempts
    for bad_word in BAD_WORDS.iter() {
        let word: &str = *bad_word;
        // Skip short words to avoid false positives
        if word.len() < 4 {
            continue;
        }
        let normalized_bad = word
            .replace(['0', '1', '3', '4', '5', '@', '!', '$', '#'], "");
        if normalized.contains(&normalized_bad) {
            return Some(word.to_string());
        }
    }

    // Check if any bad word is contained within the text (for compound words)
    for bad_word in BAD_WORDS.iter() {
        let word: &str = *bad_word;
        if word.len() >= 4 && text_lower.contains(word) {
            return Some(word.to_string());
        }
    }

    None
}

/// Check if channel name is appropriate
/// Returns Ok(()) if clean, Err with reason if not
pub fn validate_channel_name(name: &str) -> Result<(), String> {
    if contains_profanity(name).is_some() {
        return Err(
            "Channel name contains inappropriate language. Please choose a different name.".to_string()
        );
    }

    // Check minimum length
    if name.trim().len() < 2 {
        return Err("Channel name must be at least 2 characters.".to_string());
    }

    // Check maximum length (Discord limit)
    if name.len() > 100 {
        return Err("Channel name must be 100 characters or less.".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_names() {
        assert!(validate_channel_name("Gaming Lounge").is_ok());
        assert!(validate_channel_name("Chill Zone").is_ok());
        assert!(validate_channel_name("Music & Chat").is_ok());
    }

    #[test]
    fn test_profanity_detection() {
        assert!(contains_profanity("fuck").is_some());
        assert!(contains_profanity("Gaming fuck Zone").is_some());
        assert!(contains_profanity("f_u_c_k").is_some());
    }
}
