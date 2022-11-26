macro_rules! setup_exchange {
    ($ch:expr, $name:ident { kind $kind:ident queues [ $($queue_name:literal),* $(,)? ] }) => ({
        $ch.exchange_declare(
            stringify!($name),
            ::lapin::ExchangeKind::Direct,
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
