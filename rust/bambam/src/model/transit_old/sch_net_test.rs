#[cfg(test)]
mod test {
    use gtfs_structures::{Gtfs, RawGtfs};

    #[test]
    fn test_read() {
        let gtfs =
            RawGtfs::new("http://data.trilliumtransit.com/gtfs/bustang-co-us/bustang-co-us.zip")
                .unwrap();
        println!(
            "there are {} stops in the Bustang gtfs",
            gtfs.stops.as_ref().unwrap().len()
        );
        // match gtfs.stops.as_ref() {
        //     Err(e) => {
        //         println!("{}", e);
        //         ()
        //     }
        //     Ok(stops) => {
        //         for stop in stops {
        //             println!(
        //                 "{}: {} ({}, {})",
        //                 stop.id,
        //                 stop.description.clone().unwrap_or_default(),
        //                 stop.latitude.unwrap_or_default(),
        //                 stop.longitude.unwrap_or_default(),
        //             )
        //         }
        //     }
        // }
    }
}
