
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id, c.Id) AS RankScore,
        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
TopPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.Score,
        rp.CreationDate,
        rp.RankScore,
        rp.CommentCount,
        COALESCE(b.Name, 'No Badge') AS UserBadge
    FROM 
        RankedPosts rp
    LEFT JOIN 
        Badges b ON rp.PostId = b.UserId
    WHERE 
        rp.RankScore <= 5
),
ClosedPosts AS (
    SELECT 
        ph.PostId AS ClosedPostId,
        ph.CreationDate,
        ph.UserDisplayName,
        STRING_AGG(pt.Name, ', ') AS CloseReasons
    FROM 
        PostHistory ph
    JOIN 
        PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
    WHERE 
        pt.Name IN ('Post Closed', 'Post Reopened')
    GROUP BY 
        ph.PostId, ph.CreationDate, ph.UserDisplayName
)
SELECT 
    tp.PostId,
    tp.Title,
    tp.Score,
    tp.CommentCount,
    tp.UserBadge,
    cp.CloseReasons
FROM 
    TopPosts tp
LEFT JOIN 
    ClosedPosts cp ON tp.PostId = cp.ClosedPostId
WHERE 
    tp.CommentCount > 0 
ORDER BY 
    tp.Score DESC;
