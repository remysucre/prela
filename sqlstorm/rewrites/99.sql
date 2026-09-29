WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id, b.Id) AS Rank,
        COALESCE(b.Class, 0) AS BadgeClass,
        COALESCE(u.UpVotes, 0) AS UserUpVotes,
        COALESCE(u.DownVotes, 0) AS UserDownVotes
    FROM 
        Posts p
    LEFT JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        Badges b ON u.Id = b.UserId AND b.Date = (
            SELECT MAX(Date) FROM Badges WHERE UserId = u.Id
        )
    WHERE 
        p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
),
TopPosts AS (
    SELECT 
        rp.*,
        GREATEST(rp.Score + rp.UserUpVotes - rp.UserDownVotes, 0) AS AdjustedScore
    FROM 
        RankedPosts rp
    WHERE 
        rp.Rank = 1
)
SELECT 
    tp.Title,
    tp.CreationDate,
    tp.Score,
    tp.ViewCount,
    tp.AdjustedScore,
    CASE 
        WHEN tp.BadgeClass = 1 THEN 'Gold'
        WHEN tp.BadgeClass = 2 THEN 'Silver'
        WHEN tp.BadgeClass = 3 THEN 'Bronze'
        ELSE 'No Badge'
    END AS BadgeType,
    COUNT(c.Id) AS CommentCount,
    (SELECT COUNT(DISTINCT pl.RelatedPostId)
     FROM PostLinks pl
     WHERE pl.PostId = tp.Id AND pl.LinkTypeId = 3) AS DuplicateCount
FROM 
    TopPosts tp
LEFT JOIN 
    Comments c ON tp.Id = c.PostId
GROUP BY 
    tp.Id, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AdjustedScore, tp.BadgeClass
ORDER BY 
    tp.AdjustedScore DESC, tp.CreationDate DESC
LIMIT 10;