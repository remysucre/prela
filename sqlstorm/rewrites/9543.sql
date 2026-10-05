WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        u.DisplayName AS OwnerDisplayName,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1 AND 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
TopPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.OwnerDisplayName,
        rp.CreationDate,
        rp.Score,
        rp.ViewCount
    FROM 
        RankedPosts rp
    WHERE 
        rp.Rank <= 5
),
PostsWithBadges AS (
    SELECT 
        tp.PostId,
        tp.Title,
        tp.OwnerDisplayName,
        tp.CreationDate,
        tp.Score,
        tp.ViewCount,
        COUNT(b.Id) AS BadgeCount
    FROM 
        TopPosts tp
    LEFT JOIN 
        Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
    GROUP BY 
        tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount
)
SELECT 
    p.Title,
    p.OwnerDisplayName,
    p.CreationDate,
    p.Score,
    p.ViewCount,
    p.BadgeCount,
    CASE 
        WHEN p.BadgeCount >= 5 THEN 'Gold Contributor'
        WHEN p.BadgeCount >= 3 THEN 'Silver Contributor'
        WHEN p.BadgeCount >= 1 THEN 'Bronze Contributor'
        ELSE 'No Badges'
    END AS ContributionLevel
FROM 
    PostsWithBadges p
ORDER BY 
    p.Score DESC, 
    p.ViewCount DESC;