
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.Score,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
        COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
        AND p.ViewCount IS NOT NULL
),

RecentPostHistory AS (
    SELECT 
        ph.PostId,
        pht.Name AS HistoryType,
        ph.CreationDate,
        ph.UserDisplayName,
        ph.Comment,
        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank
    FROM 
        PostHistory ph
    JOIN 
        PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
    WHERE 
        ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
),

FilteredTopPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.ViewCount,
        rp.Score,
        rp.VoteCount,
        COALESCE(rp.RankScore, 0) AS RankScore, 
        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
        (SELECT STRING_AGG(tag.TagName, ', ' ORDER BY tag.TagName) 
         FROM Tags tag
         INNER JOIN Posts p ON p.Tags LIKE CONCAT('%', tag.TagName, '%')
         WHERE p.Id = rp.PostId) AS TagsUsed
    FROM 
        RankedPosts rp
    WHERE 
        rp.RankScore <= 5
)

SELECT 
    ftp.PostId,
    ftp.Title,
    ftp.ViewCount,
    ftp.Score,
    ftp.VoteCount,
    ftp.CommentCount,
    ftp.TagsUsed,
    ph.HistoryType,
    ph.UserDisplayName,
    ph.CreationDate AS HistoryCreationDate
FROM 
    FilteredTopPosts ftp
LEFT JOIN 
    RecentPostHistory ph ON ftp.PostId = ph.PostId 
WHERE 
    (ph.HistoryRank IS NULL OR ph.HistoryRank = 1)
ORDER BY 
    ftp.Score DESC, 
    ph.CreationDate DESC
LIMIT 10;
