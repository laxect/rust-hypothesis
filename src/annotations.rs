use std::collections::HashMap;

use chrono::{DateTime, Utc};
use color_eyre::Help;
use eyre::WrapErr;
use reqwest::Url;
use serde::{Deserialize, Serialize, Serializer};
#[cfg(feature = "cli")]
use structopt::StructOpt;

use crate::errors::APIError;
use crate::{is_default, Hypothesis, UserAccountID, API_URL};

impl Hypothesis {
    /// Create a new annotation
    ///
    /// Posts a new annotation object to Hypothesis.
    /// Returns an [`Annotation`](annotations/struct.Annotation.html) as output.
    /// See [`InputAnnotation`'s](annotations/struct.InputAnnotation.html) docs for examples on what you can add to an annotation.
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use hypothesis::Hypothesis;
    /// use hypothesis::annotations::InputAnnotationBuilder;
    /// #     dotenv::dotenv()?;
    /// #     let username = dotenv::var("USERNAME")?;
    /// #     let developer_key = dotenv::var("DEVELOPER_KEY")?;
    /// #     let group_id = dotenv::var("TEST_GROUP_ID").unwrap_or("__world__".into());
    ///
    /// let api = Hypothesis::new(&username, &developer_key)?;
    /// let annotation = api.create_annotation(&InputAnnotationBuilder::default()
    ///                     .text("string")
    ///                     .uri("http://example.com")
    ///                     .group(&group_id)
    ///                     .build()?).await?;
    /// assert_eq!(&annotation.text, "string");
    /// #    api.delete_annotation(&annotation.id).await?;
    /// #    Ok(())
    /// # }
    /// ```
    pub async fn create_annotation(
        &self,
        annotation: &InputAnnotation,
    ) -> color_eyre::Result<Annotation> {
        let text = self
            .client
            .post(&format!("{}/annotations", API_URL))
            .json(annotation)
            .send()
            .await?
            .text()
            .await?;
        let result = serde_json::from_str::<Annotation>(&text)
            .wrap_err(serde_json::from_str::<APIError>(&text).unwrap_or_default())
            .suggestion("Make sure input fields are valid");
        Ok(result?)
    }

    /// Update an existing annotation
    ///
    /// Change any field in an existing annotation. Returns the modified [`Annotation`](annotations/struct.Annotation.html)
    /// Fields in  [`InputAnnotation`](annotations/struct.InputAnnotation.html) which are left as default are not modified in the annotation
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use hypothesis::Hypothesis;
    /// use hypothesis::annotations::InputAnnotationBuilder;
    /// #     dotenv::dotenv()?;
    /// #     let username = dotenv::var("USERNAME")?;
    /// #     let developer_key = dotenv::var("DEVELOPER_KEY")?;
    /// #     let group_id = dotenv::var("TEST_GROUP_ID").unwrap_or("__world__".into());
    /// let api = Hypothesis::new(&username, &developer_key)?;
    /// #     let annotation = api.create_annotation(&InputAnnotationBuilder::default()
    /// #                        .text("string")
    /// #                        .uri("http://example.com")
    /// #                        .group(&group_id)
    /// #                        .build()?).await?;
    /// #    let annotation_id = annotation.id.to_owned();    
    /// let updated_annotation = api.update_annotation(&annotation_id,
    ///                             &InputAnnotationBuilder::default()
    ///                                 .tags(vec!["tag1".to_string(), "tag2".to_string()])
    ///                                 .text("New String")
    ///                                 .build()?).await?;
    ///  assert_eq!(updated_annotation.id, annotation_id);
    ///  assert_eq!(&updated_annotation.text, "New String");
    /// #    api.delete_annotation(&updated_annotation.id).await?;
    /// #    Ok(())
    /// # }
    /// ```

    pub async fn update_annotation(
        &self,
        id: &str,
        annotation: &InputAnnotation,
    ) -> color_eyre::Result<Annotation> {
        let text = self
            .client
            .patch(&format!("{}/annotations/{}", API_URL, id))
            .json(&annotation)
            .send()
            .await?
            .text()
            .await?;
        let result = serde_json::from_str::<Annotation>(&text)
            .wrap_err(serde_json::from_str::<APIError>(&text).unwrap_or_default())
            .suggestion("Make sure input fields are valid");
        Ok(result?)
    }

    /// Search for annotations with optional filters
    ///
    /// Returns a list of annotations matching the search query.
    /// See  [`SearchQuery`](annotations/struct.SearchQuery.html) for more filtering options
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use hypothesis::{Hypothesis, UserAccountID};
    /// use hypothesis::annotations::SearchQueryBuilder;
    /// #     dotenv::dotenv()?;
    /// #     let username = dotenv::var("USERNAME")?;
    /// #     let developer_key = dotenv::var("DEVELOPER_KEY")?;
    /// let api = Hypothesis::new(&username, &developer_key)?;
    /// /// Search for your own annotations:
    /// let search_query = SearchQueryBuilder::default().user(&api.user).build()?;
    /// let search_results = api.search_annotations(&search_query).await?;
    /// #     assert!(!search_results.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    pub async fn search_annotations(
        &self,
        query: &SearchQuery,
    ) -> color_eyre::Result<Vec<Annotation>> {
        let query: HashMap<String, serde_json::Value> =
            serde_json::from_str(&serde_json::to_string(&query)?)?;
        let url = Url::parse_with_params(
            &format!("{}/search", API_URL),
            &query
                .into_iter()
                .map(|(k, v)| (k, v.to_string().replace('"', "")))
                .collect::<Vec<_>>(),
        )?;
        let text = self.client.get(url).send().await?.text().await?;
        #[derive(Deserialize, Debug, Clone, PartialEq)]
        struct SearchResult {
            rows: Vec<Annotation>,
            total: usize,
        }
        let result = serde_json::from_str::<SearchResult>(&text)
            .wrap_err(serde_json::from_str::<APIError>(&text).unwrap_or_default())
            .suggestion("Make sure the query is valid");
        Ok(result?.rows)
    }

    /// Fetch annotation by ID
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use hypothesis::Hypothesis;
    /// #    use hypothesis::annotations::InputAnnotationBuilder;
    /// #    dotenv::dotenv()?;
    /// #    let username = dotenv::var("USERNAME")?;
    /// #    let developer_key = dotenv::var("DEVELOPER_KEY")?;
    /// #    let group_id = dotenv::var("TEST_GROUP_ID").unwrap_or("__world__".into());
    /// let api = Hypothesis::new(&username, &developer_key)?;
    /// #    let annotation = api.create_annotation(&InputAnnotationBuilder::default()
    /// #                       .text("string")
    /// #                       .uri("http://example.com")
    /// #                       .group(group_id).build()?).await?;
    /// #    let annotation_id = annotation.id.to_owned();    
    /// let annotation = api.fetch_annotation(&annotation_id).await?;
    /// assert_eq!(annotation.id, annotation_id);
    /// #    api.delete_annotation(&annotation.id).await?;
    /// #    Ok(())
    /// # }
    /// ```
    pub async fn fetch_annotation(&self, id: &str) -> color_eyre::Result<Annotation> {
        let text = self
            .client
            .get(&format!("{}/annotations/{}", API_URL, id))
            .send()
            .await?
            .text()
            .await?;
        let result = serde_json::from_str::<Annotation>(&text)
            .wrap_err(serde_json::from_str::<APIError>(&text).unwrap_or_default())
            .suggestion("Make sure the given String exists");
        Ok(result?)
    }

    /// Delete annotation by ID
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use hypothesis::Hypothesis;
    /// #    use hypothesis::annotations::InputAnnotationBuilder;
    /// #    dotenv::dotenv()?;
    /// #    let username = dotenv::var("USERNAME")?;
    /// #    let developer_key = dotenv::var("DEVELOPER_KEY")?;
    /// #    let group_id = dotenv::var("TEST_GROUP_ID").unwrap_or("__world__".into());
    /// let api = Hypothesis::new(&username, &developer_key)?;
    /// #    let annotation = api.create_annotation(&InputAnnotationBuilder::default()
    /// #                       .text("string")
    /// #                       .uri("http://example.com")
    /// #                       .group(group_id).build()?).await?;
    /// #    let annotation_id = annotation.id.to_owned();    
    /// let deleted = api.delete_annotation(&annotation_id).await?;
    /// assert!(deleted);
    /// assert!(api.fetch_annotation(&annotation_id).await.is_err());
    /// #    Ok(())
    /// # }
    /// ```

    pub async fn delete_annotation(&self, id: &str) -> color_eyre::Result<bool> {
        let text = self
            .client
            .delete(&format!("{}/annotations/{}", API_URL, id))
            .send()
            .await?
            .text()
            .await?;
        #[derive(Deserialize, Debug, Clone, PartialEq)]
        struct DeletionResult {
            id: String,
            deleted: bool,
        }
        let result = serde_json::from_str::<DeletionResult>(&text)
            .wrap_err(serde_json::from_str::<APIError>(&text).unwrap_or_default())
            .suggestion("Make sure the given Annotation ID exists");
        Ok(result?.deleted)
    }

    /// Flag an annotation
    ///
    /// Flag an annotation for review (moderation). The moderator of the group containing the
    /// annotation will be notified of the flag and can decide whether or not to hide the
    /// annotation. Note that flags persist and cannot be removed once they are set.
    pub async fn flag_annotation(&self, id: &str) -> color_eyre::Result<()> {
        let text = self
            .client
            .put(&format!("{}/annotations/{}/flag", API_URL, id))
            .send()
            .await?
            .text()
            .await?;
        let error = serde_json::from_str::<APIError>(&text);
        if let Ok(error) = error {
            Err(error).suggestion("Make sure the given Annotation ID exists")
        } else {
            Ok(())
        }
    }

    /// Hide an annotation
    ///
    /// Hide an annotation. The authenticated user needs to have the moderate permission for the
    /// group that contains the annotation — this permission is granted to the user who created the group.
    pub async fn hide_annotation(&self, id: &str) -> color_eyre::Result<()> {
        let text = self
            .client
            .put(&format!("{}/annotations/{}/hide", API_URL, id))
            .send()
            .await?
            .text()
            .await?;
        let error = serde_json::from_str::<APIError>(&text);
        if let Ok(error) = error {
            Err(error).suggestion("Make sure the given Annotation ID exists")
        } else {
            Ok(())
        }
    }

    /// Show an annotation
    ///
    /// Show/"un-hide" an annotation. The authenticated user needs to have the moderate permission
    /// for the group that contains the annotation—this permission is granted to the user who created the group.
    pub async fn show_annotation(&self, id: &str) -> color_eyre::Result<()> {
        let text = self
            .client
            .delete(&format!("{}/annotations/{}/hide", API_URL, id))
            .send()
            .await?
            .text()
            .await?;
        let error = serde_json::from_str::<APIError>(&text);
        if let Ok(error) = error {
            Err(error).suggestion("Make sure the given Annotation ID exists")
        } else {
            Ok(())
        }
    }
}

#[cfg_attr(feature = "cli", derive(StructOpt))]
#[cfg_attr(
    feature = "cli",
    structopt(
        about = "Create or update an annotation",
        long_about = "Create / update and upload an annotation to your Hypothesis"
    )
)]
/// Struct to create and update annotations
///
/// For creating a new annotation, all fields except uri are optional, i.e. leave as default.
/// For updating an existing annotation, all fields are optional.
///
/// # Example
/// ```
/// use hypothesis::annotations::{InputAnnotationBuilder, TargetBuilder, Selector};
/// # #[tokio::main]
/// # async fn main() -> color_eyre::Result<()> {
/// // A simple annotation
/// let annotation_simple = InputAnnotationBuilder::default()
///     .uri("https://www.example.com")
///     .text("My new annotation").build()?;
///
/// // A complex annotation
/// let annotation_complex = InputAnnotationBuilder::default()
///     .uri("https://www.example.com")
///     .text("this is a comment")
///     .target(TargetBuilder::default().source("https://www.example.com")
///         .selector(vec![Selector::new_quote("exact text in website to highlight",
///                                             "prefix of text",
///                                             "suffix of text")]).build()?)
///     .tags(vec!["tag1".into(), "tag2".into()])
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Default, Clone, Builder, PartialEq)]
#[builder(default, build_fn(name = "builder"))]
pub struct InputAnnotation {
    /// URI that this annotation is attached to.
    ///
    /// Can be a URL (a web page address) or a URN representing another kind of resource such as
    /// DOI (Digital Object Identifier) or a PDF fingerprint.
    #[serde(skip_serializing_if = "is_default")]
    #[builder(setter(into))]
    pub uri: String,
    /// Annotation text / comment given by user
    ///
    /// This is NOT the selected text on the web-page
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub text: String,
    /// Tags attached to the annotation
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(long))]
    #[builder(setter(strip_option), default)]
    pub tags: Option<Vec<String>>,
    /// Further metadata about the target document
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(skip))]
    #[builder(setter(strip_option), default)]
    pub document: Option<Document>,
    #[serde(skip_serializing_if = "is_default")]
    /// The unique identifier for the annotation's group.
    ///
    /// If an annotation is a reply to another
    /// annotation (see `references`), this field will be ignored —
    /// replies belong to the same group as their parent annotations.
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub group: String,
    /// Which part of the document does the annotation target?
    ///
    /// If left as default then the annotation is linked to the whole page.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(skip))]
    pub target: Target,
    /// Annotation IDs for any annotations this annotation references (e.g. is a reply to)
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(long))]
    pub references: Vec<String>,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Builder)]
#[builder(build_fn(name = "builder"))]
pub struct Document {
    #[serde(skip_serializing_if = "is_default")]
    pub title: Vec<String>,
    #[serde(skip_serializing_if = "is_default")]
    #[builder(setter(strip_option), default)]
    pub dc: Option<Dc>,
    #[serde(skip_serializing_if = "is_default")]
    #[builder(setter(strip_option), default)]
    pub highwire: Option<HighWire>,
    #[serde(skip_serializing_if = "is_default")]
    pub link: Vec<Link>,
}

impl DocumentBuilder {
    pub fn build(&self) -> color_eyre::Result<Document> {
        self.builder().map_err(|e| eyre!(e))
    }
}

#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct HighWire {
    #[serde(skip_serializing_if = "is_default")]
    pub doi: Vec<String>,
    #[serde(skip_serializing_if = "is_default")]
    pub pdf_url: Vec<String>,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Link {
    pub href: String,
    #[serde(skip_serializing_if = "is_default", rename = "type")]
    pub link_type: String,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Dc {
    #[serde(skip_serializing_if = "is_default")]
    pub identifier: Vec<String>,
}

impl InputAnnotationBuilder {
    pub fn build(&self) -> color_eyre::Result<InputAnnotation> {
        self.builder().map_err(|e| eyre!(e))
    }
}

/// Full representation of Annotation resource and applicable relationships.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Annotation {
    /// Annotation ID
    pub id: String,
    /// Date of creation
    pub created: DateTime<Utc>,
    /// Date of last update
    pub updated: DateTime<Utc>,
    /// User account ID in the format "acct:<username>@<authority>"
    pub user: UserAccountID,
    /// URL of document this annotation is attached to
    pub uri: String,
    /// The text content of the annotation body (NOT the selected text in the document)
    pub text: String,
    /// Tags attached to annotation
    pub tags: Vec<String>,
    /// The unique identifier for the annotation's group
    pub group: String,
    pub permissions: Permissions,
    /// Which part of the document does the annotation target.
    pub target: Vec<Target>,
    /// An object containing hypermedia links for this annotation
    pub links: HashMap<String, String>,
    /// Whether this annotation is hidden from public view
    pub hidden: bool,
    /// Whether this annotation has one or more flags for moderation
    pub flagged: bool,
    /// Annotation IDs for any annotations this annotation references (e.g. is a reply to)
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub user_info: Option<UserInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UserInfo {
    /// The annotation creator's display name
    #[serde(default)]
    pub display_name: String,
}

/// > While the API accepts arbitrary Annotation selectors in the target.selector property,
/// > the Hypothesis client currently supports TextQuoteSelector, RangeSelector and TextPositionSelector selector.
/// [Hypothesis API v1.0.0](https://h.readthedocs.io/en/latest/api-reference/v1/#tag/annotations/paths/~1annotations/post)
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Builder)]
#[builder(build_fn(name = "builder"))]
pub struct Target {
    /// The target URI for the annotation
    /// Leave empty when creating an annotation
    #[serde(skip_serializing_if = "is_default")]
    #[builder(setter(into))]
    pub source: String,
    /// An array of selectors that refine this annotation's target
    #[serde(default, skip_serializing_if = "is_default")]
    pub selector: Vec<Selector>,
}

impl TargetBuilder {
    pub fn build(&self) -> color_eyre::Result<Target> {
        self.builder().map_err(|e| eyre!(e))
    }
}

/// > Many Annotations refer to part of a resource, rather than all of it, as the Target.
/// > We call that part of the resource a Segment (of Interest). A Selector is used to describe how
/// > to determine the Segment from within the Source resource.
/// [Web Annotation Data Model - Selectors](https://www.w3.org/TR/annotation-model/#selectors)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Selector {
    TextQuoteSelector(TextQuoteSelector),
    /// > Selections made by users may be extensive and/or cross over internal boundaries in the
    /// > representation, making it difficult to construct a single selector that robustly describes
    /// > the correct content. A Range Selector can be used to identify the beginning and the end of
    /// > the selection by using other Selectors. In this way, two points can be accurately identified
    /// > using the most appropriate selection mechanisms, and then linked together to form the selection.
    /// > The selection consists of everything from the beginning of the starting selector through to the
    /// > beginning of the ending selector, but not including it.
    /// [Web Annotation Data Model - Range Selector](https://www.w3.org/TR/annotation-model/#range-selector)
    /// NOTE - the Hypothesis API doesn't seem to follow this standard for RangeSelector so this just returns a HashMap for now
    /// TODO: make RangeSelector a struct
    RangeSelector(HashMap<String, serde_json::Value>),
    TextPositionSelector(TextPositionSelector),
}

impl Selector {
    pub fn new_quote(exact: &str, prefix: &str, suffix: &str) -> Selector {
        Selector::TextQuoteSelector(TextQuoteSelector {
            exact: exact.to_string(),
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
        })
    }
}

/// > This Selector describes a range of text by copying it, and including some of the text
/// > immediately before (a prefix) and after (a suffix) it to distinguish between multiple
/// > copies of the same sequence of characters.
///
/// > For example, if the document were again "abcdefghijklmnopqrstuvwxyz", one could select
/// > "efg" by a prefix of "abcd", the match of "efg" and a suffix of "hijk".
/// [Web Annotation Data Model - Text Quote Selector](https://www.w3.org/TR/annotation-model/#text-quote-selector)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextQuoteSelector {
    /// A copy of the text which is being selected, after normalization.
    pub exact: String,
    /// A snippet of text that occurs immediately before the text which is being selected.
    pub prefix: String,
    /// The snippet of text that occurs immediately after the text which is being selected.
    pub suffix: String,
}

/// >  This Selector describes a range of text by recording the start and end positions of the
/// > selection in the stream. Position 0 would be immediately before the first character, position
/// > 1 would be immediately before the second character, and so on. The start character is thus
/// > included in the list, but the end character is not.
///
/// > For example, if the document was "abcdefghijklmnopqrstuvwxyz", the start was 4, and the end
/// > was 7, then the selection would be "efg".
/// [Web Annotation Data Model - Text Position Selector](https://www.w3.org/TR/annotation-model/#text-position-selector)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextPositionSelector {
    /// The starting position of the segment of text. The first character in the full text is
    /// character position 0, and the character is included within the segment.
    pub start: u64,
    /// The end position of the segment of text. The character is not included within the segment.
    pub end: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    Created,
    Updated,
    Id,
    Group,
    User,
}

impl Default for Sort {
    fn default() -> Self {
        Self::Updated
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Asc,
    Desc,
}

impl Default for Order {
    fn default() -> Self {
        Self::Desc
    }
}

/// See [the Hypothesis API docs](https://h.readthedocs.io/en/latest/api-reference/v1/#tag/annotations/paths/~1search/get) for more details on using these fields
#[cfg_attr(feature = "cli", derive(StructOpt))]
#[derive(Serialize, Debug, Clone, PartialEq, Builder, Default)]
#[builder(build_fn(name = "builder"), default)]
pub struct SearchQuery {
    /// The maximum number of annotations to return.
    ///
    /// Default: 20. Range: [ 0 .. 200 ]
    #[builder(default = "20")]
    #[cfg_attr(feature = "cli", structopt(default_value = "20", long))]
    pub limit: u8,
    /// The field by which annotations should be sorted
    /// One of created, updated, id, group, user
    ///
    /// Default: updated
    #[cfg_attr(feature = "cli", structopt(default_value = "updated", long, possible_values = & Sort::variants()))]
    pub sort: Sort,
    /// Example: "2019-01-03T19:46:09.334Z"
    ///
    /// Define a start point for a subset (page) of annotation search results.
    /// NOTE: make sure to set sort to `Sort::Asc` if using `search_after`
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub search_after: String,
    /// The number of initial annotations to skip in the result set.
    ///
    /// Default: 0. Range: <= 9800.
    /// search_after is more efficient.
    #[cfg_attr(feature = "cli", structopt(default_value = "0", long))]
    pub offset: usize,
    /// The order in which the results should be sorted.
    /// One of asc, desc
    ///
    /// Default: desc
    #[cfg_attr(feature = "cli", structopt(default_value = "desc", long, possible_values = & Order::variants()))]
    pub order: Order,
    /// Limit the results to annotations matching the specific URI or equivalent URIs.
    ///
    /// URI can be a URL (a web page address) or a URN representing another kind of resource such
    /// as DOI (Digital Object Identifier) or a PDF fingerprint.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub uri: String,
    /// Limit the results to annotations containing the given keyword (tokenized chunk) in the URI.
    /// The value must exactly match an individual URI keyword.
    ///
    #[serde(rename = "uri.parts", skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub uri_parts: String,
    /// Limit the results to annotations whose URIs match the wildcard pattern.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub wildcard_uri: String,
    /// Limit the results to annotations made by the specified user. (in the format `acct:<username>@<authority>`)
    #[serde(skip_serializing_if = "is_default", serialize_with = "serialize_user")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub user: UserAccountID,
    /// Limit the results to annotations made in the specified group (by group ID).
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub group: String,
    /// Limit the results to annotations tagged with the specified value.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub tag: String,
    /// Similar to tag but allows a list of multiple tags.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(long))]
    pub tags: Vec<String>,
    /// Limit the results to annotations who contain the indicated keyword in any of the following fields:
    /// `quote`, `tags`, `text`, `url`
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub any: String,
    /// Limit the results to annotations that contain this text inside the text that was annotated.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub quote: String,
    /// Returns annotations that are replies to this parent annotation ID.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub references: String,
    /// Limit the results to annotations that contain this text in their textual body.
    #[serde(skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "cli", structopt(default_value, long))]
    #[builder(setter(into))]
    pub text: String,
}

impl SearchQueryBuilder {
    pub fn build(&self) -> color_eyre::Result<SearchQuery> {
        self.builder().map_err(|e| eyre!(e))
    }
}

fn serialize_user<S>(x: &UserAccountID, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&x.0)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Permissions {
    pub read: Vec<String>,
    pub delete: Vec<String>,
    pub admin: Vec<String>,
    pub update: Vec<String>,
}
