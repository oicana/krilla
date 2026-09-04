//! Custom XMP metadata.
//!
//! In addition to the standard properties exposed on [`Metadata`], krilla
//! supports writing arbitrary XMP metadata under user-defined namespaces.
//! This is useful for embedding things like e-invoice descriptors
//! (e.g. ZUGFeRD/Factur-X) or RDF license information into a PDF.
//!
//! Build a [`Property`] with a [`Namespace`] and a [`Value`], then attach
//! the properties via [`Metadata::custom_xmp_properties`].
//!
//! ## Predefined schemas
//!
//! If a namespace URI matches a schema known to xmp-writer,
//! properties are written under that schema's canonical prefix, and the
//! namespace's prefix is ignored.
//! It is your responsibility that the property actually exists in the
//! predefined schema.
//!
//! Custom namespaces must not reuse the prefix of a predefined schema or
//! bind one prefix to two different URIs, and all declarations of the same
//! URI must be identical; otherwise [`Document::finish`] returns an
//! [`XmpError`].
//!
//! ## PDF/A
//!
//! Any property in a custom namespace must be described in a PDF/A
//! extension schema. Populate [`Namespace::property_descriptions`] with a
//! [`PropertyDescription`] for every property you write.
//!
//! [`Document::finish`]: crate::document::Document::finish
//! [`Metadata`]: super::Metadata
//! [`Metadata::custom_xmp_properties`]: super::Metadata::custom_xmp_properties

use super::DateTime;

/// An XMP namespace.
///
/// A namespace is identified by its URI. The prefix is the short name used
/// in the serialized XML. For PDF/A output, the
/// namespace must describe each property through
/// [`Self::property_descriptions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    /// The XML prefix (e.g. `"fx"`).
    ///
    /// Ignored if the URI is natively known to krilla.
    pub prefix: String,
    /// The namespace URI (e.g. `"urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#"`).
    pub uri: String,
    /// Optional human-readable schema name for the PDF/A extension schema
    /// description. Defaults to `"<prefix> schema"`.
    pub schema_name: Option<String>,
    /// Optional human-readable description of the schema.
    pub description: Option<String>,
    /// PDF/A extension schema property descriptions.
    ///
    /// Required for any property name written under this namespace
    /// when exporting to PDF/A.
    pub property_descriptions: Vec<PropertyDescription>,
}

impl Namespace {
    /// Create a new namespace with the given prefix and URI.
    pub fn new(prefix: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            uri: uri.into(),
            schema_name: None,
            description: None,
            property_descriptions: Vec::new(),
        }
    }

    /// Set the human-readable schema name used for the PDF/A extension
    /// schema description.
    pub fn schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = Some(name.into());
        self
    }

    /// Set the human-readable description of the schema.
    pub fn schema_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a property description used for the PDF/A extension schema.
    pub fn add_description(mut self, description: PropertyDescription) -> Self {
        self.property_descriptions.push(description);
        self
    }
}

/// Description of a single XMP property under a [`Namespace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDescription {
    /// The property name (e.g. `"DocumentType"`).
    pub name: String,
    /// The type of the property's value.
    pub value_type: ValueType,
    /// Whether the property is generated internally by the producer or
    /// supplied externally by the user.
    pub category: Category,
    /// Whether the property must be present.
    ///
    /// PDF/A extension schemas cannot express this, so it is only used when
    /// describing the metadata in RELAX NG.
    pub required: bool,
    /// Human-readable description of the property.
    pub description: String,
}

impl PropertyDescription {
    /// Create a new property description.
    pub fn new(
        name: impl Into<String>,
        value_type: ValueType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value_type,
            category: Category::External,
            required: false,
            description: description.into(),
        }
    }

    /// Mark the property as being generated internally by the producer.
    pub fn internal(mut self) -> Self {
        self.category = Category::Internal;
        self
    }

    /// Mark the property as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// The type of an XMP value.
///
/// This mirrors the XMP type system: the simple types of the XMP
/// specification, the three generic array types and language alternatives.
/// Structures cannot be described yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// A simple value.
    Simple(SimpleType),
    /// A value drawn from an open-ended set of recommended values.
    OpenChoice(SimpleType),
    /// A value drawn from a fixed set. Contains the allowed values.
    ClosedChoice(SimpleType, Vec<String>),
    /// An ordered array (`rdf:Seq`) of the contained type.
    OrderedArray(Box<ValueType>),
    /// An unordered array (`rdf:Bag`) of the contained type.
    UnorderedArray(Box<ValueType>),
    /// An alternative array (`rdf:Alt`) of the contained type.
    AlternativeArray(Box<ValueType>),
    /// A language alternative: an `rdf:Alt` of `xml:lang`-tagged text.
    LanguageAlternative,
}

impl ValueType {
    /// An ordered array (`rdf:Seq`) of the given type.
    pub fn ordered_array(item: impl Into<ValueType>) -> Self {
        Self::OrderedArray(Box::new(item.into()))
    }

    /// An unordered array (`rdf:Bag`) of the given type.
    pub fn unordered_array(item: impl Into<ValueType>) -> Self {
        Self::UnorderedArray(Box::new(item.into()))
    }

    /// An alternative array (`rdf:Alt`) of the given type.
    pub fn alternative_array(item: impl Into<ValueType>) -> Self {
        Self::AlternativeArray(Box::new(item.into()))
    }

    /// A value drawn from a fixed set.
    pub fn closed_choice(
        ty: SimpleType,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::ClosedChoice(ty, values.into_iter().map(Into::into).collect())
    }
}

impl From<SimpleType> for ValueType {
    fn from(value: SimpleType) -> Self {
        Self::Simple(value)
    }
}

/// A simple XMP value type.
///
/// `Text`, `Boolean`, `Integer`, `Real` and `Date` are the core simple types;
/// the rest are derived from them and are all stored as text, but carry
/// additional meaning.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SimpleType {
    /// Unconstrained text.
    Text,
    /// A boolean, written as `True` or `False`.
    Boolean,
    /// An integer.
    Integer,
    /// A floating-point number.
    Real,
    /// A date and time.
    Date,
    /// The name of an agent, i.e. a piece of software.
    AgentName,
    /// A globally unique identifier.
    Guid,
    /// A language tag.
    Locale,
    /// A MIME type.
    MimeType,
    /// The name of a person or organization.
    ProperName,
    /// The name of a rendition of a document.
    RenditionClass,
    /// A URI.
    Uri,
    /// A URL.
    Url,
    /// A rational number, written as `numerator/denominator`.
    Rational,
    /// A frame rate, written as `f<frames>` or `f<frames>s<basis>`.
    FrameRate,
    /// A number of frames at a given frame rate.
    FrameCount,
    /// A path identifying a portion of a resource, e.g. `/content/audio`.
    Part,
    /// An XPath expression.
    XPath,
}

impl SimpleType {
    /// The name of the type in a PDF/A extension schema description.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Real => "Real",
            Self::Date => "Date",
            Self::AgentName => "AgentName",
            Self::Guid => "GUID",
            Self::Locale => "Locale",
            Self::MimeType => "MIMEType",
            Self::ProperName => "ProperName",
            Self::RenditionClass => "RenditionClass",
            Self::Uri => "URI",
            Self::Url => "URL",
            Self::Rational => "Rational",
            Self::FrameRate => "FrameRate",
            Self::FrameCount => "FrameCount",
            Self::Part => "Part",
            Self::XPath => "XPath",
        }
    }
}

/// Whether a property is generated internally or supplied externally.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Category {
    /// The property is computed by the producer (e.g. page count).
    Internal,
    /// The property is supplied by the user.
    External,
}

/// A single XMP property attached to a [`Namespace`].
///
/// Also used for the fields of a [`Value::Struct`], which are named values in
/// a namespace just like top-level properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// The namespace this property belongs to.
    pub namespace: Namespace,
    /// The property name within the namespace.
    pub name: String,
    /// The property value.
    pub value: Value,
}

impl Property {
    /// Create a new XMP property.
    pub fn new(namespace: Namespace, name: impl Into<String>, value: Value) -> Self {
        Self {
            namespace,
            name: name.into(),
            value,
        }
    }
}

/// An XMP property value.
///
/// The variants mirror the value shapes supported by XMP/RDF: primitive
/// types, the three array kinds (`rdf:Seq`, `rdf:Bag`, `rdf:Alt`), language
/// alternatives, and structs.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A text value.
    Text(String),
    /// A boolean value.
    Bool(bool),
    /// An integer value.
    Integer(i64),
    /// A floating-point value. Must be finite.
    Real(f64),
    /// A date value.
    Date(DateTime),
    /// An ordered array (`rdf:Seq`).
    OrderedArray(Vec<Value>),
    /// An unordered array (`rdf:Bag`).
    UnorderedArray(Vec<Value>),
    /// An alternative array (`rdf:Alt`).
    AlternativeArray(Vec<Value>),
    /// A language-alternative array (`rdf:Alt` of `xml:lang`-tagged text).
    ///
    /// Each entry pairs an optional RFC 3066 language tag (`None` ⇒
    /// `x-default`) with its text value.
    LanguageAlternative(Vec<(Option<String>, String)>),
    /// A struct value (`rdf:parseType="Resource"`), given as its fields.
    Struct(Vec<Property>),
}

impl Value {
    /// Convenience constructor for a [`Value::Text`].
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Convenience constructor for a [`Value::Date`].
    pub fn date(value: DateTime) -> Self {
        Self::Date(value)
    }
}

/// An invalid set of custom XMP properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmpError {
    /// The same namespace URI was declared inconsistently: all declarations
    /// of one URI must be identical, but two of them disagreed on the prefix,
    /// schema name, or property descriptions. Contains the namespace URI.
    ConflictingNamespace(String),
    /// One prefix was bound to two different namespace URLs. Contains the
    /// prefix.
    ConflictingPrefix(String),
    /// A custom namespace used a prefix reserved by a predefined XMP schema
    /// (e.g. `dc`, `xmp`, `pdf`). Contains the prefix.
    ReservedPrefix(String),
    /// A [`Value::Real`] was NaN or infinite, which can't be represented in
    /// XMP. Contains the name of the containing property.
    NonFiniteReal(String),
}

impl std::fmt::Display for XmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmpError::ConflictingNamespace(uri) => {
                write!(
                    f,
                    "the namespace {uri} was declared with multiple different prefixes"
                )
            }
            XmpError::ConflictingPrefix(prefix) => {
                write!(
                    f,
                    "the prefix {prefix} was bound to two different namespaces"
                )
            }
            XmpError::ReservedPrefix(prefix) => {
                write!(
                    f,
                    "the prefix {prefix} is reserved by a predefined XMP schema"
                )
            }
            XmpError::NonFiniteReal(name) => {
                write!(f, "the property {name} contained a non-finite real number")
            }
        }
    }
}

impl std::error::Error for XmpError {}
