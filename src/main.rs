use anyhow::{Context, Result, anyhow};
use rand::prelude::*;
use std::io;
use std::process;

mod memmap;
use memmap::*;

fn main() -> Result<()> {
    let pagesize = match nix::unistd::sysconf(nix::unistd::SysconfVar::PAGE_SIZE) {
        Ok(Some(val)) => val as u64,
        Ok(None) => {
            return Err(anyhow!(
                "Page size is unlimited??? That's gotta be a big TLB!"
            ));
        }
        Err(x) => return Err(x).context("Failed to get page size"),
    };
    println!("Page size is {} bytes", pagesize);

    let mut rng = rand::rng();

    let my_pid = process::id();

    println!("Enumerating mappings and setting them to writeable...");
    for mapping in &mut get_memmap(my_pid)? {
        println!("{}", mapping);
        if mapping.path.starts_with("[") {
            println!(
                "  Skipping {} - special page",
                mapping.path
            );
            continue;
        }
        mapping.set_permissions(PermissionSet::from(&"rwxp"))?;
    }
    println!("");

    println!("New mappings:");
    let mappings = get_memmap(my_pid)?;
    for mapping in &mappings {
        println!("{}", mapping);
    }

    let candidates = &mappings
        .into_iter()
        .filter(|m| !m.path.starts_with("["))
        .collect::<Vec<Mapping>>();

    println!(
        "\nWelcome to Rust Roulette! It's a daring game where pages of memory are overwritten "
    );
    println!("until something terrible happens!\n");
    println!("'Some of you may die, but it's a sacrifice I'm willing to make' - J. Lithgow\n");
    loop {
        println!("Are you still feeling lucky? Press ENTER to play 1 more round");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Waiting for user input");

        let mapping = &candidates.choose(&mut rng).unwrap();
        let page_count = mapping.size() / pagesize;
        let page_idx = rng.random_range(0..page_count);
        let start_addr = mapping.start_addr + pagesize * page_idx;
        let end_addr = start_addr + pagesize;

        println!(
            "Bang! {} @ 0x{:X} - 0x{:X}\n",
            mapping
                .path
                .split('/')
                .last()
                .expect("failed to spilt path"),
            start_addr,
            end_addr
        );
        for addr in start_addr..end_addr {
            let addr = addr as *mut u8;
            unsafe {
                *addr = 0xFF;
            }
        }
    }

    // Previously-used functionality where a NOP slide was written inside of libc's executable
    // mapping and then we jumped into it.
    //
    //println!("Building a NOP slide in the first page of the region");
    //println!("Page 0x{:x} -> 0x00", mapping.start_addr);
    //for addr in mapping.start_addr..mapping.start_addr + pagesize {
    //    let addr = addr as *mut u8;
    //    unsafe {
    //        *addr = 0x0;
    //    }
    //}
    //for mapping in new_mappings {
    //    println!("{}", mapping);
    //    println!("Press ENTER to jump on the NOP sled I stuck in libc!");
    //    let mut input = String::new();
    //    io::stdin()
    //        .read_line(&mut input)
    //        .expect("Waiting for user input");
    //    unsafe {
    //        // This allows us to pivot execution into the page data we just wrote. This is the "turn it
    //        // up to 11" of unsafe blocks.
    //        let func: extern "C" fn() = std::mem::transmute(mapping.start_addr as *mut u8);
    //        func();
    //    }
    //}
}
