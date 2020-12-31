# school_things
Make learning & school open source

## Running
This assumes running on Ubuntu
1. Install postgres `sudo apt install postgres`
2. Install rust/rustup `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Install diesel_cli `cargo install diesel_cli` (depending on your environment, there may be errors; refer to [diesel.rs](https://diesel.rs/guides/getting-started/) & google to help you)
4. Install git `sudo apt install git`
5. Navigate to desired parent folder
    - I like to structure my git repositories like `~/git/github/<org or user>/<repo>`
6. Clone repository `git clone https://github.com/ClaytonDoesThings/school_things.git`
7. Enter repository folder `cd school_things`
8. Setup database
    1. Enter postgres user `sudo su - postgres`
    2. Enter postgres CLI `psql`
    3. Set postgres database password `ALTER USER postgres PASSWORD 'desiredPassword';`
    4. Exit CLI `exit`
    5. Create database `createdb school_things;`
    6. Exit postgres user `exit`
9. Setup diesel environment
    1. Create `.env` file in repository root (Make sure )  
        `DATABASE_URL=postgres://postgres:<postgres password>@localhost/school_things`
    2. Run diesel setup `diesel setup`
    3. Run migrations `diesel migration run`
10. Setup rocket.rs config
    1. Create `Rocket.toml` file in repository root (remember to replace `<>`)  
        ```toml
        [development]
        address = "0.0.0.0"
        secret_key = "<output of running `openssl rand -base64 32`>"

        [global.databases]
        postgres = { url = "postgres://postgres:<poastgres password>@localhost/school_things" }
        ```
11. Run `cargo run`