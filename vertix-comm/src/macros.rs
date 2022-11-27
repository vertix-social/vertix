/// Initialize an exchange on ``$ch`, named `$name`, with the given kind and queue names
///
/// ```no_run
/// #async fn ctx(ch: &lapin::Channel) -> crate::error::Result<()> {
/// setup_exchange!(ch,
///     TestExchange {
///         kind Direct
///         queues ["TestExchange.process", "TestExchange.send"]
///     }
/// );
/// #}
/// ```
macro_rules! setup_exchange {
    ($ch:expr, $name:ident { kind $kind:ident queues [ $($queue_name:literal),* $(,)? ] }) => ({
        $ch.exchange_declare(
            stringify!($name),
            ::lapin::ExchangeKind::$kind,
            ::lapin::options::ExchangeDeclareOptions {
                durable: true, ..Default::default() },
            Default::default()
        ).await?;

        $(
            $ch.queue_declare(
                $queue_name,
                ::lapin::options::QueueDeclareOptions {
                    durable: true, ..Default::default() },
                Default::default()
            ).await?;

            $ch.queue_bind(
                $queue_name,
                stringify!($name),
                "",
                Default::default(),
                Default::default()
            ).await?;
        )*
    });
}

pub(crate) use setup_exchange;

/// Expect a reply that matches given pattern(s). Returns a [Result], with
/// [crate::Error::InvalidReply] on failure to match.
///
/// ```
/// let reply = "foo";
/// assert!(expect_reply_of!(reply, "foo" => ()).is_ok());
/// assert!(expect_reply_of!(reply, "bar" => ()).is_err());
/// ```
#[macro_export]
macro_rules! expect_reply_of {
    ($reply:expr; $($pattern:pat => $return:expr),+ $(,)?) => (
        match $reply {
            $($pattern => ::std::result::Result::Ok($return)),+,
            _ => ::std::result::Result::Err(
                $crate::Error::InvalidReply(
                    ::std::borrow::Cow::Borrowed(
                        concat!("reply did not match pattern(s): ",
                            $(stringify!($pattern), ", "),+)))
            )
        }
    )
}
