WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        ps.Name AS PostType,
        t.TagName,
        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY v.CreationDate DESC, v.Id DESC, t.TagName) AS RecentVoteRank
    FROM 
        Posts p
    LEFT JOIN 
        PostTypes ps ON p.PostTypeId = ps.Id
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        LATERAL (
            SELECT 
                unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName
        ) t ON TRUE
    WHERE 
        p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' 
        AND p.ViewCount > 100
),

AggregatedPostMetrics AS (
    SELECT 
        rp.PostId,
        rp.Title,
        COUNT(v.Id) AS VoteCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
        MIN(rp.CreationDate) AS FirstSeen,
        MAX(rp.CreationDate) AS LastActive,
        STRING_AGG(DISTINCT rp.TagName, ', ') AS Tags
    FROM 
        RankedPosts rp
    LEFT JOIN 
        Votes v ON rp.PostId = v.PostId
    WHERE 
        rp.RecentVoteRank = 1
    GROUP BY 
        rp.PostId, rp.Title
)

SELECT 
    apm.PostId,
    apm.Title,
    apm.VoteCount,
    apm.UpvoteCount,
    apm.DownvoteCount,
    apm.FirstSeen,
    apm.LastActive,
    apm.Tags,
    (SELECT COUNT(*) FROM Comments c WHERE c.PostId = apm.PostId) AS CommentCount
FROM 
    AggregatedPostMetrics apm
ORDER BY 
    apm.VoteCount DESC, apm.LastActive DESC
LIMIT 100;