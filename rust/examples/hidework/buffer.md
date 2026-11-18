Implement a load_mut method to take advantage of the non-materialization optimization with .with() and like a right load method to go with it, and then run the benchmarks on it. 
need to pay attention to the to_memory() method overhead and fix it.
Read more PAX to figure out what's up with Parquet vs PAX.
Run more Job queries to get data.



movie.with(kind.eq("Crime")).group_by(year).select(title)

year.inv().with(movie.with(kind.eq("Crime"))).select(title)

year.inv().with(movie).select(title)

year.inv().select(title)


year.inv().with(kind.eq("Crime")).select(title)

Movie.by(year).with(kind.eq("Crime")).select(title)

Load for Inv()

movie.group_by(cast.select("name"))
                    select      
    with    
                       title

inv       kind.eq("Crime")
            row_ids

year

key_filter, value filter
row_ids, years_present



TODO:
- create a specialised left and right loader where we only apply filters on one of the columns instead 


def select:
    b = self.val_load(filter=val_filter) # only reads the val_col
    a = self.key_load(filter=key_filter) # this only reads the column of the keys self

-- but this gets at the current structure of returning Multirel's for Prela to do... this would be nice for with's which don't need to be materialised but 