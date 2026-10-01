
WITH RankedPosts AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        COALESCE(v.VoteCount, 0) AS Score,
        COALESCE(c.CommentCount, 0) AS Comments,
        COALESCE(b.BadgeCount, 0) AS BadgeCount,
        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(v.VoteCount, 0) DESC, COALESCE(c.CommentCount, 0) DESC) AS Rank
    FROM
        Posts p
    LEFT JOIN (
        SELECT
            PostId,
            COUNT(*) AS VoteCount
        FROM
            Votes
        GROUP BY PostId
    ) v ON p.Id = v.PostId
    LEFT JOIN (
        SELECT
            PostId,
            COUNT(*) AS CommentCount
        FROM
            Comments
        GROUP BY PostId
    ) c ON p.Id = c.PostId
    LEFT JOIN (
        SELECT
            UserId,
            COUNT(*) AS BadgeCount
        FROM
            Badges
        GROUP BY UserId
    ) b ON p.OwnerUserId = b.UserId
),
PostHistoryDetail AS (
    SELECT
        ph.PostId,
        ph.PostHistoryTypeId,
        pht.Name AS HistoryType,
        ph.CreationDate
    FROM
        PostHistory ph
    JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
    WHERE
        ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
        AND ph.Comment IS NOT NULL
),
FilteredPosts AS (
    SELECT
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        rp.Score,
        rp.Comments,
        rp.BadgeCount,
        rp.Rank,
        COUNT(pd.PostId) AS RecentHistoryCount,
        STRING_AGG(DISTINCT pd.HistoryType, ', ' ORDER BY pd.HistoryType) AS RecentHistoryTypes
    FROM
        RankedPosts rp
    LEFT JOIN PostHistoryDetail pd ON rp.PostId = pd.PostId
    GROUP BY
        rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Comments, rp.BadgeCount, rp.Rank
)

SELECT
    fp.PostId,
    fp.Title,
    fp.CreationDate,
    fp.Score,
    fp.Comments,
    fp.BadgeCount,
    fp.Rank,
    CASE
        WHEN fp.RecentHistoryCount = 0 THEN 'No Recent Edits'
        ELSE fp.RecentHistoryTypes
    END AS Edits_Info
FROM
    FilteredPosts fp
WHERE
    fp.Rank <= 5
    AND (fp.Score > 0 OR fp.Comments > 0)
ORDER BY
    fp.Score DESC,
    fp.Comments DESC;
