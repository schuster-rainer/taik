//! The conversation of a job. It lives in the job folder, beside the brief and the rounds.
//!
//! A message is one create-only file `message-NNN.user.md` or `message-NNN.agent.md`. Its first line
//! says how many rounds existed when it was written. A reply of the user to a finished job grants
//! one further round, and the worker of that round reads the whole conversation. A worker may ask a
//! question instead of delivering. Then the job waits until the user has answered.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Result, record};

pub const MAX_MESSAGE_BYTES: usize = 8192;
const MAX_MESSAGES: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Author {
    User,
    Agent,
}

impl Author {
    fn suffix(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub author: Author,
    /// Round records that existed when the message was written.
    pub round: usize,
    pub text: String,
}

/// All messages of a job, oldest first. A file that cannot be read as a message is skipped.
pub fn read(job: &Path) -> Vec<Message> {
    let mut messages = Vec::new();
    for sequence in 1..=MAX_MESSAGES {
        let found = [Author::User, Author::Agent]
            .into_iter()
            .find_map(|author| {
                let path = job.join(format!("message-{sequence:03}.{}.md", author.suffix()));
                let text = record::read_regular(&path, MAX_MESSAGE_BYTES + 64).ok()?;
                let (head, body) = text.split_once("\n\n")?;
                let round = head.strip_prefix("round=")?.trim().parse().ok()?;
                Some(Message {
                    author,
                    round,
                    text: body.trim().to_owned(),
                })
            });
        match found {
            Some(message) => messages.push(message),
            None => break,
        }
    }
    messages
}

/// Appends a message. Published like a record: complete or absent, never replaced.
pub fn write(job: &Path, author: Author, round: usize, text: &str) -> Result<PathBuf> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_MESSAGE_BYTES {
        return Err("a message is between 1 and 8192 bytes".into());
    }
    let sequence = (1..=MAX_MESSAGES)
        .find(|sequence| {
            [Author::User, Author::Agent].into_iter().all(|author| {
                !job.join(format!("message-{sequence:03}.{}.md", author.suffix()))
                    .exists()
            })
        })
        .ok_or("this job holds the maximum number of messages")?;
    let path = job.join(format!("message-{sequence:03}.{}.md", author.suffix()));
    record::publish_text(
        &path,
        &format!("round={round}\n\n{text}\n"),
        MAX_MESSAGE_BYTES + 64,
    )?;
    Ok(path)
}

/// A worker asked and nobody has answered yet.
pub fn waits_for_user(messages: &[Message]) -> bool {
    messages
        .last()
        .is_some_and(|message| message.author == Author::Agent)
}

/// The user wrote last and no round has run since.
pub fn open_reply(messages: &[Message], rounds: usize) -> bool {
    messages
        .last()
        .is_some_and(|message| message.author == Author::User && message.round >= rounds)
}

/// How many further rounds the user's replies grant beyond the budget of a job.
pub fn granted_rounds(messages: &[Message]) -> usize {
    messages
        .iter()
        .filter(|message| message.author == Author::User)
        .count()
        .min(8)
}

/// The conversation as the part of a worker prompt.
pub fn prompt(messages: &[Message]) -> String {
    let mut text = String::new();
    if messages.is_empty() {
        return text;
    }
    text.push_str("\n\nCONVERSATION ABOUT THIS WORK ORDER, OLDEST FIRST:\n");
    for message in messages {
        let who = match message.author {
            Author::User => "user",
            Author::Agent => "you",
        };
        text.push_str(&format!("[{who}] {}\n", message.text));
    }
    if messages
        .last()
        .is_some_and(|message| message.author == Author::User)
    {
        text.push_str("\nFollow the last message of the user in this round.");
    }
    text
}

/// The number of round records in a job folder.
pub fn rounds_done(job: &Path) -> usize {
    fs::read_dir(job).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                // round-NN.run only. A ledger such as round-NN.analysis.run is no round.
                name.len() == "round-00.run".len()
                    && name.starts_with("round-")
                    && name.ends_with(".run")
            })
            .count()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conversation_is_kept_in_order_and_decides_who_is_next() {
        let folder = tempfile::tempdir().unwrap();
        let job = folder.path();
        assert!(read(job).is_empty());
        assert!(!waits_for_user(&[]) && !open_reply(&[], 0) && prompt(&[]).is_empty());

        write(job, Author::Agent, 0, "Welche Farbe?").unwrap();
        let messages = read(job);
        assert!(waits_for_user(&messages) && !open_reply(&messages, 0));

        write(job, Author::User, 0, "  Blau.\n").unwrap();
        let messages = read(job);
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[1],
            Message {
                author: Author::User,
                round: 0,
                text: "Blau.".into()
            }
        );
        assert!(!waits_for_user(&messages) && open_reply(&messages, 0));
        // A round ran since: the reply is answered.
        assert!(!open_reply(&messages, 1));
        assert_eq!(granted_rounds(&messages), 1);
        let asked = prompt(&messages);
        assert!(
            asked.contains("[you] Welche Farbe?\n[user] Blau.\n"),
            "{asked}"
        );
        assert!(asked.ends_with("Follow the last message of the user in this round."));

        assert!(write(job, Author::User, 0, "   ").is_err());
        assert!(write(job, Author::User, 0, &"x".repeat(MAX_MESSAGE_BYTES + 1)).is_err());
        assert!(
            job.join("message-001.agent.md").is_file() && job.join("message-002.user.md").is_file()
        );
        assert_eq!(rounds_done(job), 0);
        fs::write(job.join("round-01.run"), "x").unwrap();
        fs::write(job.join("round-01.analysis.run"), "x").unwrap();
        assert_eq!(rounds_done(job), 1, "the ledger of an analysis is no round");
    }
}
