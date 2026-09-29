WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
),
RecentVotes AS (
    SELECT 
        v.PostId,
        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
    FROM 
        Votes v
    WHERE 
        v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month'
    GROUP BY 
        v.PostId
),
PostWithRating AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.ViewCount,
        rv.UpVotes,
        rv.DownVotes,
        COALESCE(rv.UpVotes - rv.DownVotes, 0) AS Score,
        rp.Rank
    FROM 
        RankedPosts rp
    LEFT JOIN 
        RecentVotes rv ON rp.PostId = rv.PostId
)
SELECT 
    pwr.PostId,
    pwr.Title,
    pwr.ViewCount,
    pwr.UpVotes,
    pwr.DownVotes,
    pwr.Score,
    CASE 
        WHEN pwr.Score > 0 THEN 'Positive'
        WHEN pwr.Score < 0 THEN 'Negative'
        ELSE 'Neutral'
    END AS VoteStatus,
    CASE 
        WHEN pwr.Rank <= 5 THEN 'Top Post'
        ELSE 'Regular Post'
    END AS PostCategory
FROM 
    PostWithRating pwr
WHERE 
    pwr.Score <> 0
ORDER BY 
    pwr.Score DESC, pwr.ViewCount DESC;