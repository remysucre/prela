
WITH PostStats AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.OwnerUserId,
        p.CreationDate,
        COUNT(DISTINCT c.Id) AS CommentCount,
        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        Badges b ON p.OwnerUserId = b.UserId
    WHERE 
        p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '30 days')
    GROUP BY 
        p.Id, p.Title, p.OwnerUserId, p.CreationDate
),
PopularPosts AS (
    SELECT 
        ps.PostId,
        ps.Title,
        ps.CommentCount,
        ps.UpVoteCount,
        ps.DownVoteCount,
        ps.OwnerUserId,
        ps.CreationDate,
        ps.UserPostRank
    FROM 
        PostStats ps
    WHERE 
        ps.CommentCount > 5
        OR ps.UpVoteCount > 10
        OR (ps.DownVoteCount = 0 AND ps.CommentCount >= 3)
),
LatestPost AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate
    FROM 
        Posts p
    WHERE 
        p.CreationDate = (SELECT MAX(CreationDate) FROM Posts)
),
PostLinksDetails AS (
    SELECT 
        pl.PostId,
        pl.RelatedPostId,
        lt.Name AS LinkType
    FROM 
        PostLinks pl
    JOIN 
        LinkTypes lt ON pl.LinkTypeId = lt.Id
)
SELECT 
    pp.PostId,
    pp.Title,
    pp.CommentCount,
    pp.UpVoteCount,
    pp.DownVoteCount,
    u.DisplayName AS UserDisplayName,
    CASE 
        WHEN u.Reputation IS NULL THEN 'No Reputation Data'
        ELSE CAST(u.Reputation AS VARCHAR) || ' Reputation Points'
    END AS UserReputation,
    COALESCE(lp.Title, 'No Recent Post') AS LatestPostTitle,
    COALESCE(pld.LinkType, 'No Links') AS LinkType,
    EXTRACT(EPOCH FROM (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - pp.CreationDate)) / 60 AS AgeInMinutes
FROM 
    PopularPosts pp
LEFT JOIN 
    Users u ON pp.OwnerUserId = u.Id
LEFT JOIN 
    LatestPost lp ON pp.PostId = lp.Id
LEFT JOIN 
    PostLinksDetails pld ON pp.PostId = pld.PostId
ORDER BY 
    pp.UpVoteCount DESC, pp.CommentCount DESC, pp.PostId, pld.LinkType
OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
