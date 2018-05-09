
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Net(::siege_net::Error, ::siege_net::ErrorKind);
        Mesh(::siege_mesh::Error, ::siege_mesh::ErrorKind);
        Ddsfile(::ddsfile::Error, ::ddsfile::ErrorKind);
        Render(::siege_render::Error, ::siege_render::ErrorKind);
    }

    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        Addr(::std::net::AddrParseError);
        TomlDe(::toml::de::Error);
        SetLogger(::log::SetLoggerError);
        WinitCreation(::winit::CreationError);
        Dacite(::dacite::core::Error);
        DaciteEarly(::dacite::core::EarlyInstanceError);
        DaciteWinit(::dacite_winit::Error);
        Bincode(::bincode::Error);
    }

    errors {
        General(s: String) {
            description("General Error"),
            display("General Error: '{}'", s),
        }
    }
}
