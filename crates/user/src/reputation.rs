//! Reputation and review system for user accounts.
//!
//! Reputation is calculated based on:
//! - Membership duration (longer members get higher base rating)
//! - Number of successful trades as maker and taker
//! - Reviews from trading partners (1-5 star ratings)

use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use opendex_types::TradeId;

use crate::error::UserError;
use crate::user::UserId;

/// Star rating for a review (1-5 stars).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rating(u32);

impl Rating {
    /// Minimum rating.
    pub const MIN: u32 = 1;

    /// Maximum rating.
    pub const MAX: u32 = 5;

    /// Create a new rating.
    pub fn new(value: u32) -> Result<Self, UserError> {
        if value < Self::MIN || value > Self::MAX {
            return Err(UserError::InvalidRating);
        }
        Ok(Self(value))
    }

    /// Get the rating value.
    pub fn value(&self) -> u32 {
        self.0
    }

    /// Check if this is a positive rating (4-5 stars).
    pub fn is_positive(&self) -> bool {
        self.0 >= 4
    }

    /// Check if this is a negative rating (1-2 stars).
    pub fn is_negative(&self) -> bool {
        self.0 <= 2
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}★", self.0)
    }
}

/// A review left by one user about another after a trade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// Unique review identifier.
    pub id: Uuid,
    /// Trade this review is for.
    pub trade_id: TradeId,
    /// User who wrote the review.
    pub reviewer_id: UserId,
    /// User being reviewed.
    pub reviewed_id: UserId,
    /// Star rating (1-5).
    pub rating: Rating,
    /// Optional text comment.
    pub comment: Option<String>,
    /// Review creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Review {
    /// Create a new review.
    pub fn new(
        trade_id: TradeId,
        reviewer_id: UserId,
        reviewed_id: UserId,
        rating: Rating,
        comment: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            trade_id,
            reviewer_id,
            reviewed_id,
            rating,
            comment,
            created_at: Utc::now(),
        }
    }
}

/// Reputation level based on score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationLevel {
    /// New member, 0-9 reputation points.
    Newbie,
    /// Established member, 10-49 points.
    Member,
    /// Trusted trader, 50-99 points.
    Trusted,
    /// Veteran trader, 100-249 points.
    Veteran,
    /// Elite trader, 250+ points.
    Elite,
}

impl ReputationLevel {
    /// Calculate reputation level from score.
    pub fn from_score(score: u32) -> Self {
        match score {
            0..=9 => ReputationLevel::Newbie,
            10..=49 => ReputationLevel::Member,
            50..=99 => ReputationLevel::Trusted,
            100..=249 => ReputationLevel::Veteran,
            _ => ReputationLevel::Elite,
        }
    }
}

impl fmt::Display for ReputationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReputationLevel::Newbie => write!(f, "Newbie"),
            ReputationLevel::Member => write!(f, "Member"),
            ReputationLevel::Trusted => write!(f, "Trusted"),
            ReputationLevel::Veteran => write!(f, "Veteran"),
            ReputationLevel::Elite => write!(f, "Elite"),
        }
    }
}

/// Complete reputation profile for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationProfile {
    /// User ID.
    pub user_id: UserId,
    /// Total reputation score.
    pub score: u32,
    /// Reputation level.
    pub level: ReputationLevel,
    /// Average review rating (1.0-5.0).
    pub average_rating: f64,
    /// Total number of reviews received.
    pub review_count: usize,
    /// Membership duration in days.
    pub member_days: u64,
    /// Reviews received by this user.
    pub reviews: Vec<Review>,
}

impl ReputationProfile {
    /// Create a new empty reputation profile.
    pub fn new(user_id: UserId, member_days: u64) -> Self {
        // Base reputation from membership duration: 1 point per 30 days, max 50
        let membership_score = ((member_days / 30).min(50)) as u32;

        Self {
            user_id,
            score: membership_score,
            level: ReputationLevel::from_score(membership_score),
            average_rating: 0.0,
            review_count: 0,
            member_days,
            reviews: Vec::new(),
        }
    }

    /// Add a review to the profile and recalculate score.
    pub fn add_review(&mut self, review: Review) {
        self.reviews.push(review);
        self.recalculate();
    }

    /// Recalculate the reputation score and level.
    pub fn recalculate(&mut self) {
        // Base score from membership (1 point per 30 days, max 50)
        let membership_score = ((self.member_days / 30).min(50)) as u32;

        // Review score: average rating * number of reviews / 2, max 100
        let review_score = if self.reviews.is_empty() {
            0
        } else {
            let avg: f64 = self
                .reviews
                .iter()
                .map(|r| r.rating.value() as f64)
                .sum::<f64>()
                / self.reviews.len() as f64;
            self.average_rating = avg;
            (avg * self.reviews.len() as f64 / 2.0) as u32
        };

        // Trade completion bonus: 1 point per 10 reviews (as proxy for completed trades)
        let trade_bonus = (self.reviews.len() as u32 / 10).min(50);

        self.score = membership_score + review_score + trade_bonus;
        self.review_count = self.reviews.len();
        self.level = ReputationLevel::from_score(self.score);
    }

    /// Get the average rating as a string (e.g., "4.5/5.0").
    pub fn average_rating_str(&self) -> String {
        if self.reviews.is_empty() {
            "No reviews".to_string()
        } else {
            format!("{:.1}/5.0", self.average_rating)
        }
    }
}

/// Service for managing reviews and reputation.
pub struct ReputationService {
    /// Reputation profiles for all users.
    profiles: HashMap<UserId, ReputationProfile>,
    /// Set of trades that already have reviews (to prevent double reviews).
    reviewed_trades: HashMap<TradeId, UserId>, // trade_id -> reviewer_id
}

impl ReputationService {
    /// Create a new reputation service.
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            reviewed_trades: HashMap::new(),
        }
    }

    /// Register a user's reputation profile.
    pub fn register_profile(&mut self, user_id: UserId, member_days: u64) {
        let profile = ReputationProfile::new(user_id, member_days);
        self.profiles.insert(user_id, profile);
    }

    /// Submit a review for a trading partner.
    pub fn submit_review(
        &mut self,
        trade_id: TradeId,
        reviewer_id: UserId,
        reviewed_id: UserId,
        rating: Rating,
        comment: Option<String>,
    ) -> Result<Review, UserError> {
        // Cannot review yourself
        if reviewer_id == reviewed_id {
            return Err(UserError::SelfReview);
        }

        // Check if this trade already has a review from this reviewer
        if let Some(&existing_reviewer) = self.reviewed_trades.get(&trade_id) {
            if existing_reviewer == reviewer_id {
                return Err(UserError::ReviewAlreadySubmitted);
            }
        }

        // Ensure both users have profiles
        if !self.profiles.contains_key(&reviewed_id) {
            self.register_profile(reviewed_id, 0);
        }

        // Create the review
        let review = Review::new(trade_id, reviewer_id, reviewed_id, rating, comment);

        // Add to the reviewed user's profile
        if let Some(profile) = self.profiles.get_mut(&reviewed_id) {
            profile.add_review(review.clone());
        }

        // Mark this trade as reviewed by this user
        self.reviewed_trades.insert(trade_id, reviewer_id);

        Ok(review)
    }

    /// Get a user's reputation profile.
    pub fn get_profile(&self, user_id: UserId) -> Option<&ReputationProfile> {
        self.profiles.get(&user_id)
    }

    /// Get a user's reputation score.
    pub fn get_score(&self, user_id: UserId) -> u32 {
        self.profiles
            .get(&user_id)
            .map(|p| p.score)
            .unwrap_or(0)
    }

    /// Get a user's reputation level.
    pub fn get_level(&self, user_id: UserId) -> ReputationLevel {
        self.profiles
            .get(&user_id)
            .map(|p| p.level)
            .unwrap_or(ReputationLevel::Newbie)
    }

    /// Update membership days for a user and recalculate.
    pub fn update_membership_days(&mut self, user_id: UserId, days: u64) {
        if let Some(profile) = self.profiles.get_mut(&user_id) {
            profile.member_days = days;
            profile.recalculate();
        }
    }

    /// Record a completed trade for a user (increases reputation slightly).
    pub fn record_completed_trade(&mut self, user_id: UserId) {
        if let Some(profile) = self.profiles.get_mut(&user_id) {
            // Small bonus for completed trades
            profile.score = profile.score.saturating_add(1);
            profile.level = ReputationLevel::from_score(profile.score);
        }
    }
}

impl Default for ReputationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_validation() {
        assert!(Rating::new(1).is_ok());
        assert!(Rating::new(5).is_ok());
        assert!(Rating::new(0).is_err());
        assert!(Rating::new(6).is_err());
    }

    #[test]
    fn rating_positive_negative() {
        assert!(Rating::new(5).unwrap().is_positive());
        assert!(Rating::new(4).unwrap().is_positive());
        assert!(Rating::new(2).unwrap().is_negative());
        assert!(Rating::new(1).unwrap().is_negative());
    }

    #[test]
    fn reputation_level_from_score() {
        assert_eq!(ReputationLevel::from_score(0), ReputationLevel::Newbie);
        assert_eq!(ReputationLevel::from_score(9), ReputationLevel::Newbie);
        assert_eq!(ReputationLevel::from_score(10), ReputationLevel::Member);
        assert_eq!(ReputationLevel::from_score(50), ReputationLevel::Trusted);
        assert_eq!(ReputationLevel::from_score(100), ReputationLevel::Veteran);
        assert_eq!(ReputationLevel::from_score(250), ReputationLevel::Elite);
    }

    #[test]
    fn membership_reputation() {
        let user_id = UserId::new();

        // New member — 0 days
        let profile = ReputationProfile::new(user_id, 0);
        assert_eq!(profile.score, 0);
        assert_eq!(profile.level, ReputationLevel::Newbie);

        // 90 days — 3 points
        let profile = ReputationProfile::new(user_id, 90);
        assert_eq!(profile.score, 3);

        // 1500 days (4+ years) — capped at 50
        let profile = ReputationProfile::new(user_id, 1500);
        assert_eq!(profile.score, 50);
    }

    #[test]
    fn submit_review() {
        let mut service = ReputationService::new();
        let alice = UserId::new();
        let bob = UserId::new();
        service.register_profile(alice, 60);
        service.register_profile(bob, 30);

        let trade_id = TradeId(Uuid::new_v4());
        let rating = Rating::new(5).unwrap();

        let review = service
            .submit_review(trade_id, alice, bob, rating, Some("Great trader!".to_string()))
            .unwrap();

        assert_eq!(review.reviewer_id, alice);
        assert_eq!(review.reviewed_id, bob);
        assert_eq!(review.rating, rating);

        let profile = service.get_profile(bob).unwrap();
        assert_eq!(profile.review_count, 1);
        assert!((profile.average_rating - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cannot_review_yourself() {
        let mut service = ReputationService::new();
        let alice = UserId::new();
        service.register_profile(alice, 60);

        let result = service.submit_review(
            TradeId(Uuid::new_v4()),
            alice,
            alice,
            Rating::new(5).unwrap(),
            None,
        );
        assert_eq!(result.unwrap_err(), UserError::SelfReview);
    }

    #[test]
    fn cannot_review_same_trade_twice() {
        let mut service = ReputationService::new();
        let alice = UserId::new();
        let bob = UserId::new();
        service.register_profile(alice, 60);
        service.register_profile(bob, 30);

        let trade_id = TradeId(Uuid::new_v4());
        service
            .submit_review(trade_id, alice, bob, Rating::new(5).unwrap(), None)
            .unwrap();

        let result = service.submit_review(
            trade_id,
            alice,
            bob,
            Rating::new(4).unwrap(),
            None,
        );
        assert_eq!(result.unwrap_err(), UserError::ReviewAlreadySubmitted);
    }

    #[test]
    fn reputation_score_calculation() {
        let mut service = ReputationService::new();
        let alice = UserId::new();
        let bob = UserId::new();
        service.register_profile(alice, 60);
        service.register_profile(bob, 365); // 1 year = 12 membership points

        // Alice gives Bob 5 reviews with 5 stars each
        for _ in 0..5 {
            service
                .submit_review(
                    TradeId(Uuid::new_v4()),
                    alice,
                    bob,
                    Rating::new(5).unwrap(),
                    None,
                )
                .unwrap();
        }

        let profile = service.get_profile(bob).unwrap();
        assert_eq!(profile.review_count, 5);
        assert!((profile.average_rating - 5.0).abs() < f64::EPSILON);
        // Score: 12 (membership) + 12.5 (reviews: 5.0 * 5 / 2) + 0 (trade bonus: 5/10) = 24
        assert!(profile.score >= 20);
    }

    #[test]
    fn record_completed_trade() {
        let mut service = ReputationService::new();
        let alice = UserId::new();
        service.register_profile(alice, 0);

        let initial_score = service.get_score(alice);
        service.record_completed_trade(alice);
        assert_eq!(service.get_score(alice), initial_score + 1);
    }

    #[test]
    fn reputation_level_display() {
        assert_eq!(format!("{}", ReputationLevel::Newbie), "Newbie");
        assert_eq!(format!("{}", ReputationLevel::Member), "Member");
        assert_eq!(format!("{}", ReputationLevel::Trusted), "Trusted");
        assert_eq!(format!("{}", ReputationLevel::Veteran), "Veteran");
        assert_eq!(format!("{}", ReputationLevel::Elite), "Elite");
    }

    #[test]
    fn average_rating_string() {
        let user_id = UserId::new();
        let profile = ReputationProfile::new(user_id, 0);
        assert_eq!(profile.average_rating_str(), "No reviews");

        let mut profile = ReputationProfile::new(user_id, 0);
        profile.average_rating = 4.5;
        profile.review_count = 1;
        profile.reviews.push(Review::new(
            TradeId(Uuid::new_v4()),
            UserId::new(),
            user_id,
            Rating::new(4).unwrap(),
            None,
        ));
        assert_eq!(profile.average_rating_str(), "4.5/5.0");
    }
}
