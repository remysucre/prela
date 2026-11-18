This is a plan outline for the non-materialised loader which should be specialised 

- cases where we only care about what values rather than keys 
- cases where we only care about surviving keys from predicates but not actualy loading in the data (e.g. for a with clause)


***In a with A.with(B) clause, we only care about keeping the parts of A whose values match with keys surviving B -- literally a semijoin reduction


There's kinda two use-cases -- where the RighLoader just means -- don't materialize the right relation but just tell me which keys survived (its LHS). I think that you should still read and use as much info as possible which will require predicates so we'll still be reading columns, just not materializing them... So we should return two filters in case... and then which values survive..


Desired Use cases 
With:
    - we don't care about getting a Bloom filter or anything like that on with, we actually just want an exact bitmap filter on the predicates 
    - however, we still need to read both columns in this case because we usually apply predicates on the thing... We can make it specialised like 
        LoadLeft for COlumnRef:
            - Consider movie.with(company.eq("A")). Then we care about what movie keys are contained in company -- so we should only read the keys of company, construct an (exact) filter on those keys, *not* load the actual company column into the sink, but just return the keys as asked
        
        LoadRight for a.select(b):
            - Let's assume that this select clause is nested inside like c.with(a.select(b)) -- at the end of the day, we only care about what keys of a survive because this is what c needs to know. To figure this out, select should load b -- 





Can actually keep much of the structure but just need to add a method that returns a bitset as an equivalent MultiRel rather than the actual relation... values don't even matter can just be default whatever.


### Design Plan for KeyLoad

Trait KeyLoad:
    KeyLoad(Self, keyfilter, valuefilter) - >KeyFilter::Exact


KeyFIlter::Exact -- is a **VecRel** (not a MultiRel) which will be created based on the keys and also an empty /default rel column like a "1" or something si that it is something that is passed and used as a filter *but* also something that can participate in the loaded relation's like Restrict{a,b} where b is KeyFilter::Exact

KeyLoad for ColumnRef(keyfilter, valuefilter):
    - if preds and valuefilter is not none, then load it as a ColumnFilter object for scan but don't pass it in as something that needs to be materialized into can like "rows" -- only the self.key-col is that 
    - do not populate sink at all 
    - Construct a KeyFilter::Exact by just pushing in exactly every key that was read into the hashmap/ not bloom filter which can have false positives 
    - return the KeyFIlter::Exact (it is actually just a VecRel)

KeyLoad for Compose:
    b_keys_exact = self.b.KeyLoad(::All, ValueFilter, Preds)
    a_keys_exact = a.KeyLoad(KeyFIlter, b_keys_exact) // we need support for passing a ::Exact object into a COlumnFIlter object just like Blooms and predicates are pushed/converted
    return a_keys_exact 

KeyLoad for Restrict
    // analogous 

**Load** for Restrict -- here we modify what the actual current TrialLoader is doing
    Load for Restrict:
        b_keys_exact = self.b.KeyLoad()
        a_rel = self.a.Load(KeyfIlter, ValFilter+b_keys_exact, preds)
        return Restrict {a_rel, b_keys_exact}



Current idea:
- design the functionality for Passing around an KeyFilter::Exact which is a bitset : it will be column IDs mapping to themselves just the identity mapping





Need to create a test suite that verifies that we can do two things:

-- run a query and then call .load().drive() USING THE TRIALLOADER and verify that this output is identical to running loaded, = query.load()
and then do db_to_memory USING THE LEFTRIGHTLOADER and then run the output withOUT the clause because it should be already applied. 

To do this, define at least 10 queries on the full IMDB dataset to test this and compare the outputs by diffing. In addition, 